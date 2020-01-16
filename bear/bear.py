#!/usr/bin/env @BEAR_PYTHON_EXECUTABLE@
# -*- coding: utf-8 -*-

# Copyright (C) 2012-2019 by László Nagy
# This file is part of Bear.
#
# Bear is a tool to generate compilation database for clang tooling.
#
# Bear is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Bear is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.
""" This module is responsible to capture the compiler invocation of any
build process. The result of that should be a compilation database.

This implementation is using the LD_PRELOAD or DYLD_INSERT_LIBRARIES
mechanisms provided by the dynamic linker. The related library is implemented
in C language and can be found under 'libear' directory.

The 'libear' library is capturing all child process creation and logging the
relevant information about it into separate files in a specified directory.
The input of the library is therefore the output directory which is passed
as an environment variable.

This module implements the build command execution with the 'libear' library
and the post-processing of the output files, which will condensates into a
(might be empty) compilation database. """

import argparse
import collections
import subprocess
import json
import sys
import functools
import os
import os.path
import re
import shlex
import itertools
import tempfile
import shutil
import struct
import contextlib
import logging

# Map of ignored compiler option for the creation of a compilation database.
# This map is used in _split_command method, which classifies the parameters
# and ignores the selected ones. Please note that other parameters might be
# ignored as well.
#
# Option names are mapped to the number of following arguments which should
# be skipped.
IGNORED_FLAGS = {
    # preprocessor macros, ignored because would cause duplicate entries in
    # the output (the only difference would be these flags). this is actual
    # finding from users, who suffered longer execution time caused by the
    # duplicates.
    '-MD': 0,
    '-MMD': 0,
    '-MG': 0,
    '-MP': 0,
    '-MF': 1,
    '-MT': 1,
    '-MQ': 1,
    # linker options, ignored because for compilation database will contain
    # compilation commands only. so, the compiler would ignore these flags
    # anyway. the benefit to get rid of them is to make the output more
    # readable.
    '-static': 0,
    '-shared': 0,
    '-s': 0,
    '-rdynamic': 0,
    '-l': 1,
    '-L': 1,
    '-u': 1,
    '-z': 1,
    '-T': 1,
    '-Xlinker': 1,
    # clang-cl / msvc cl specific flags
    # consider moving visual studio specific warning flags also
    '-nologo': 0,
    '-EHsc': 0,
    '-EHa': 0

}  # type: Dict[str, int]


# Known C/C++ compiler wrapper name patterns.
COMPILER_PATTERN_WRAPPER = re.compile(r'^(distcc|ccache)$')

# Known MPI compiler wrapper name patterns.
COMPILER_PATTERNS_MPI_WRAPPER = re.compile(
    r'^mpi(cc|cxx|CC|c\+\+|fort|f77|f90)$')

# Known C compiler executable name patterns.
COMPILER_PATTERNS_CC = (
    re.compile(r'^([^-]*-)*[mg]cc(-?\d+(\.\d+){0,2})?$'),
    re.compile(r'^([^-]*-)*clang(-\d+(\.\d+){0,2})?$'),
    re.compile(r'^(|i)cc$'),
    re.compile(r'^(g|)xlc$'),
)

# Known C++ compiler executable name patterns.
COMPILER_PATTERNS_CXX = (
    re.compile(r'^(c\+\+|cxx|CC)$'),
    re.compile(r'^([^-]*-)*[mg]\+\+(-?\d+(\.\d+){0,2})?$'),
    re.compile(r'^([^-]*-)*clang\+\+(-\d+(\.\d+){0,2})?$'),
    re.compile(r'^icpc$'),
    re.compile(r'^(g|)xl(C|c\+\+)$'),
)

# Known Fortran compiler executable name patterns
# Gfortran, Intel Fortran Compilers, PGI compilers
COMPILER_PATTERNS_FORTRAN = (
    re.compile(r'^(f95)$'),
    re.compile(r'^(gfortran)$'),
    re.compile(r'^(ifort)$'),
    re.compile(r'^(pg)(f77|f90|f95|fortran)$')
)

TRACE_FILE_PREFIX = 'execution.'  # same as in ear.c

C_LANG, CPLUSPLUS_LANG, FORTRAN_LANG, OTHER = range(4)

Execution = collections.namedtuple('Execution', ['cwd', 'cmd'])

CompilationCommand = collections.namedtuple(
    'CompilationCommand',
    ['compiler', 'language', 'phase', 'flags', 'files', 'output'])


class Tools:
    def __init__(self, only_use, c_compilers, cxx_compilers,
                 fortran_compilers):
        self.ignore = only_use
        self.c_compilers = [os.path.basename(cc) for cc in c_compilers]
        self.cxx_compilers = [os.path.basename(cc) for cc in cxx_compilers]
        self.fortran_compilers = [
            os.path.basename(cc) for cc in fortran_compilers]

    @classmethod
    def is_wrapper(cls, cmd):
        # type: (Type[Tools], str) -> bool
        return True if COMPILER_PATTERN_WRAPPER.match(cmd) else False

    @classmethod
    def is_mpi_wrapper(cls, cmd):
        # type: (Type[Tools], str) -> bool
        return True if COMPILER_PATTERNS_MPI_WRAPPER.match(cmd) else False

    def is_c_compiler(self, cmd):
        # type: (Tools, str) -> bool
        use_match = Tools._is_sting_match(cmd, self.c_compilers)
        pattern_match = Tools._is_pattern_match(cmd, COMPILER_PATTERNS_CC)
        return use_match if self.ignore else (use_match or pattern_match)

    def is_cxx_compiler(self, cmd):
        # type: (Tools, str) -> bool
        use_match = Tools._is_sting_match(cmd, self.cxx_compilers)
        pattern_match = Tools._is_pattern_match(cmd, COMPILER_PATTERNS_CXX)
        return use_match if self.ignore else (use_match or pattern_match)

    def is_fortran_compiler(self, cmd):
        # type: (Tools, str) -> bool
        use_match = Tools._is_sting_match(cmd, self.fortran_compilers)
        pattern_match = Tools._is_pattern_match(
            cmd, COMPILER_PATTERNS_FORTRAN)
        return use_match if self.ignore else (use_match or pattern_match)

    @classmethod
    def _is_sting_match(cls, candidate, compilers):
        # type (Type[Tools], str, Iterable[str) -> bool
        return any((candidate == compiler) for compiler in compilers)

    @classmethod
    def _is_pattern_match(cls, candidate, patterns):
        return any(pattern.match(candidate) for pattern in patterns)


def shell_split(string):
    # type: (str) -> List[str]
    """ Takes a command string and returns as a list. """

    def unescape(arg):
        # type: (str) -> str
        """ Gets rid of the escaping characters. """

        if len(arg) >= 2 and arg[0] == arg[-1] and arg[0] == '"':
            return re.sub(r'\\(["\\])', r'\1', arg[1:-1])
        return re.sub(r'\\([\\ $%&\(\)\[\]\{\}\*|<>@?!])', r'\1', arg)

    return [unescape(token) for token in shlex.split(string)]


def run_build(command, *args, **kwargs):
    # type: (...) -> int
    """ Run and report build command execution

    :param command: list of tokens
    :return: exit code of the process
    """
    environment = kwargs.get('env', os.environ)
    logging.debug('run build %s, in environment: %s', command, environment)
    exit_code = subprocess.call(command, *args, **kwargs)
    logging.debug('build finished with exit code: %d', exit_code)
    return exit_code


def run_command(command, cwd=None):
    # type: (List[str], str) -> List[str]
    """ Run a given command and report the execution.

    :param command: array of tokens
    :param cwd: the working directory where the command will be executed
    :return: output of the command
    """
    def decode_when_needed(result):
        # type: (Any) -> str
        """ check_output returns bytes or string depend on python version """
        if not isinstance(result, str):
            return result.decode('utf-8')
        return result

    try:
        directory = os.path.abspath(cwd) if cwd else os.getcwd()
        logging.debug('exec command %s in %s', command, directory)
        output = subprocess.check_output(command,
                                         cwd=directory,
                                         stderr=subprocess.STDOUT)
        return decode_when_needed(output).splitlines()
    except subprocess.CalledProcessError as ex:
        ex.output = decode_when_needed(ex.output).splitlines()
        raise ex


def reconfigure_logging(verbose_level):
    """ Reconfigure logging level and format based on the verbose flag.

    :param verbose_level: number of `-v` flags received by the command
    :return: no return value
    """
    # exit when nothing to do
    if verbose_level == 0:
        return

    root = logging.getLogger()
    # tune level
    level = logging.WARNING - min(logging.WARNING, (10 * verbose_level))
    root.setLevel(level)
    # be verbose with messages
    if verbose_level <= 3:
        fmt_string = '%(name)s: %(levelname)s: %(message)s'
    else:
        fmt_string = '%(name)s: %(levelname)s: %(funcName)s: %(message)s'
    handler = logging.StreamHandler(sys.stdout)
    handler.setFormatter(logging.Formatter(fmt=fmt_string))
    root.handlers = [handler]


def command_entry_point(function):
    # type: (Callable[[], int]) -> Callable[[], int]
    """ Decorator for command entry methods.

    The decorator initialize/shutdown logging and guard on programming
    errors (catch exceptions).

    The decorated method can have arbitrary parameters, the return value will
    be the exit code of the process. """

    @functools.wraps(function)
    def wrapper(*args, **kwargs):
        """ Do housekeeping tasks and execute the wrapped method. """

        try:
            logging.basicConfig(format='%(name)s: %(message)s',
                                level=logging.WARNING,
                                stream=sys.stdout)
            # this hack to get the executable name as %(name)
            logging.getLogger().name = os.path.basename(sys.argv[0])
            return function(*args, **kwargs)
        except KeyboardInterrupt:
            logging.warning('Keyboard interrupt')
            return 130  # signal received exit code for bash
        except (OSError, subprocess.CalledProcessError):
            logging.exception('Internal error.')
            if logging.getLogger().isEnabledFor(logging.DEBUG):
                logging.error("Please report this bug and attach the output "
                              "to the bug report")
            else:
                logging.error("Please run this command again and turn on "
                              "verbose mode (add '-vvvv' as argument).")
            return 64  # some non used exit code for internal errors
        finally:
            logging.shutdown()

    return wrapper


@command_entry_point
def intercept_build():
    # type: () -> int
    """ Entry point for 'intercept-build' command. """

    args = parse_args_for_intercept_build()
    tools = Tools(args.use_only, args.use_cc,
                  args.use_cxx, args.use_fortran)
    exit_code, current = capture(args, tools)

    # To support incremental builds, it is desired to read elements from
    # an existing compilation database from a previous run.
    if args.append and os.path.isfile(args.cdb):
        previous = CompilationDatabase.load(args.cdb, tools)
        entries = iter(set(itertools.chain(previous, current)))
        CompilationDatabase.save(entries, args.cdb, args.field_output)
    else:
        CompilationDatabase.save(current, args.cdb, args.field_output)

    return exit_code


def capture(args, tools):
    """ Implementation of compilation database generation.

    :param args:        the parsed and validated command line arguments
    :param tools:    helper object to detect compiler
    :return:            the exit status of build process. """

    with temporary_directory(prefix='intercept-') as tmp_dir:
        # run the build command
        environment = setup_environment(args, tmp_dir)
        exit_code = run_build(args.build, env=environment)
        # read the intercepted exec calls
        calls = (parse_exec_trace(file) for file in exec_trace_files(tmp_dir))
        safe_calls = (x for x in calls if x is not None)
        current = compilations(safe_calls, tools)
        # filter out not desired entries
        include_filter = include(args.include, args.exclude)
        filtered = set(entry for entry in current if include_filter(entry))
        return exit_code, iter(filtered)


def include(includes, excludes):
    # type: (str, str) -> Callable[[Compilation], bool]
    """ Create a predicate to filter out Compilation entries.

    :param includes: list of directories to include.
    :param excludes: list of directories to exclude.
    :return: a predicate which returns true if the entry should be
    in the final output based on the location of the source file. """

    def make_absolute(directory):
        # type: (str) -> str
        """ Makes a path like object absolute (to the project root). """

        if os.path.isabs(directory):
            return directory
        else:
            return os.path.normpath(os.path.join(os.getcwd(), directory))

    include_dirs = [make_absolute(directory) for directory in includes]
    exclude_dirs = [make_absolute(directory) for directory in excludes]

    def include_filter(candidate):
        # type: (Compilation) -> bool
        """ The predicate which returns true if the compilation should
        be included in the final output. """

        def contains(container, directory):
            # type: (str, str) -> bool
            """ Returns true if the container contains the directory.  """

            return os.path.commonprefix([container, directory]) == container

        source = candidate.source
        needed = True if len(include_dirs) == 0 else \
            any(contains(directory, source) for directory in include_dirs)
        rejected = False if len(exclude_dirs) == 0 else not \
            all(not contains(directory, source) for directory in exclude_dirs)
        return needed and not rejected

    return include_filter


def compilations(exec_calls, tools):
    # type: (Iterable[Execution], Tools) -> Iterable[Compilation]
    """ Needs to filter out commands which are not compiler calls. And those
    compiler calls shall be compilation (not pre-processing or linking) calls.
    Plus needs to find the source file name from the arguments.

    :param exec_calls:  iterator of executions
    :param tools:       helper object to detect compiler
    :return: stream of formatted compilation database entries """

    for call in exec_calls:
        for compilation in Compilation.iter_from_execution(call, tools):
            yield compilation


def setup_environment(args, destination):
    # type: (argparse.Namespace, str) -> Dict[str, str]
    """ Sets up the environment for the build command.

    In order to capture the sub-commands (executed by the build process),
    it needs to prepare the environment. It's either the compiler wrappers
    shall be announce as compiler or the intercepting library shall be
    announced for the dynamic linker.

    :param args:        command line arguments
    :param destination: directory path for the execution trace files
    :return: a prepared set of environment variables. """

    environment = dict(os.environ)
    environment.update({'INTERCEPT_BUILD_TARGET_DIR': destination})

    if sys.platform == 'darwin':
        environment.update({
            'DYLD_INSERT_LIBRARIES': args.libear,
            'DYLD_FORCE_FLAT_NAMESPACE': '1'
        })
    else:
        environment.update({'LD_PRELOAD': args.libear})

    return environment


def parse_exec_trace(filename):
    # type: (str) -> Optional[Execution]
    """ Parse execution report file.

    Given filename points to a file which contains the basic report
    generated by the interception library or compiler wrapper.

    :param filename: path to an execution trace file to read from,
    :return: an Execution object. """

    def byte_to_int(byte):
        return struct.unpack_from("=I", byte)[0]

    def parse_length(handler, expected_type):
        type_bytes = handler.read(3)
        if type_bytes != expected_type:
            raise Exception("type not expected")
        length_bytes = handler.read(4)
        return byte_to_int(length_bytes)

    def parse_string(handler):
        length = parse_length(handler, b'str')
        value_bytes = handler.read(length)
        return value_bytes.decode("utf-8")

    def parse_string_list(handler):
        length = parse_length(handler, b'lst')
        return [parse_string(handler) for _ in range(length)]

    logging.debug('parse exec trace file: %s', filename)
    with open(filename, 'rb', buffering=0) as handler:
        try:
            return Execution(cwd=parse_string(handler),
                             cmd=parse_string_list(handler))
        except Exception as exception:
            logging.warning('parse exec trace file: %s FAILED: %s',
                            filename, exception)
            return None


def exec_trace_files(directory):
    """ Generates exec trace file names.

    :param directory:   path to directory which contains the trace files.
    :return:            a generator of file names (absolute path). """

    candidates = (os.path.join(directory, file)
                  for file in os.listdir(directory)
                  if file.startswith(TRACE_FILE_PREFIX))
    return sorted((f for f in filter(os.path.isfile, candidates)),
                  key=os.path.getctime)


def parse_args_for_intercept_build():
    """ Parse and validate command-line arguments for intercept-build. """

    parser = create_intercept_parser()
    args = parser.parse_args()

    reconfigure_logging(args.verbose)
    logging.debug('Raw arguments %s', sys.argv)

    # short validation logic
    if not args.build:
        parser.error(message='missing build command')

    logging.debug('Parsed arguments: %s', args)
    return args


def create_intercept_parser():
    """ Creates a parser for command-line arguments to 'intercept'. """

    parser = argparse.ArgumentParser(
        formatter_class=argparse.ArgumentDefaultsHelpFormatter)

    parser.add_argument(
        '--version',
        action='version',
        version='%(prog)s @BEAR_VERSION@')
    parser.add_argument(
        '--verbose', '-v',
        action='count',
        default=0,
        help="""Enable verbose output from '%(prog)s'. A second, third and
        fourth flags increases verbosity.""")
    parser.add_argument(
        '--cdb', '-o',
        metavar='<file>',
        default="compile_commands.json",
        help="""The JSON compilation database.""")
    parser.add_argument(
        '--field-output',
        action='store_true',
        help="""Puts output field to entries if it founds.""")
    parser.add_argument(
        '--use-cc',
        metavar='<path>',
        dest='use_cc',
        action='append',
        default=[os.getenv('CC', 'cc')],
        help="""Hint '%(prog)s' to classify the given program name as C
        compiler.""")
    parser.add_argument(
        '--use-c++',
        metavar='<path>',
        dest='use_cxx',
        action='append',
        default=[os.getenv('CXX', 'c++')],
        help="""Hint '%(prog)s' to classify the given program name as C++
        compiler.""")
    parser.add_argument(
        '--use-fortran',
        metavar='<path>',
        dest='use_fortran',
        action='append',
        default=['f95'],
        help="""Hint '%(prog)s' to classify the given program name as Fortan
        compiler.""")
    parser.add_argument(
        '--use-only',
        action='store_true',
        help="""Only use compilers given to '--use-cc', '--use-c++' and
        '--use-fortran'.""")
    parser.add_argument(
        '--include',
        action='append',
        default=[],
        help="""Only include these directories or files to the output.
        (Absolute or relative to current working directory.)
        Use --exclude parameters to filter more entries out.""")
    parser.add_argument(
        '--exclude',
        action='append',
        default=[],
        help="""Exclude these directories or files from the output.
        (Absolute or relative to current working directory.)
        The --include will not enable entries from these directories.""")

    advanced = parser.add_argument_group('advanced options')
    advanced.add_argument(
        '--append', '-a',
        action='store_true',
        help="""Extend existing compilation database with new entries.
        Duplicate entries are detected and not present in the final output.
        The output is not continuously updated, it's done when the build
        command finished. """)
    advanced.add_argument(
        '--libear', '-l',
        dest='libear',
        default="@DEFAULT_PRELOAD_FILE@",
        action='store',
        help="""specify libear file location.""")

    parser.add_argument(
        dest='build', nargs=argparse.REMAINDER, help="""Command to run.""")
    return parser


class Compilation:
    def __init__(self,
                 compiler, language, phase, flags, source, directory, output):
        """ Constructor for a single compilation.

        This method just normalize the paths and initialize values. """

        self.compiler = compiler
        self.language = language
        self.phase = phase
        self.flags = flags
        self.directory = os.path.normpath(directory)
        self.source = source if os.path.isabs(source) else \
            os.path.normpath(os.path.join(self.directory, source))
        self.output = output

    def __hash__(self):
        # type: (Compilation) -> int
        return hash(str(self.as_dict()))

    def __eq__(self, other):
        # type: (Compilation, object) -> bool
        return (
                self.__class__ == other.__class__ and
                self.as_dict() == other.as_dict()
        )

    def as_dict(self):
        # type: (Compilation) -> Dict[str, str]
        """ This method dumps the object attributes into a dictionary. """

        candidate = vars(self).copy()
        candidate.pop("compiler", None)
        candidate.pop("output", None)
        candidate.pop("language", None)
        return candidate

    def as_db_entry(self, field_output):
        # type: (Compilation, bool) -> Dict[str, Any]
        """ This method creates a compilation database entry. """

        source = os.path.relpath(self.source, self.directory)
        if self.output:
            result = {
                'file': source,
                'arguments':
                    [self.compiler, self.phase] + self.flags +
                    ['-o', self.output] + [source],
                'directory': self.directory,
            }
            if field_output:
                result.update({'output': self.output})
            return result
        else:
            return {
                'file': source,
                'arguments':
                    [self.compiler, self.phase] + self.flags + [source],
                'directory': self.directory
            }

    @classmethod
    def from_db_entry(cls, entry, tools):
        # type: (Type[Compilation], Dict[str, str]) -> Iterable[Compilation]
        """ Parser method for compilation entry.

        From compilation database entry it creates the compilation object.

        :param entry:   the compilation database entry
        :param tools:   helper object to detect compiler
        :return: stream of CompilationDbEntry objects """

        command = shell_split(entry['command']) if 'command' in entry else \
            entry['arguments']
        execution = Execution(cmd=command, cwd=entry['directory'])
        return cls.iter_from_execution(execution, tools)

    @classmethod
    def iter_from_execution(cls, execution, tools):
        """ Generator method for compilation entries.

        From a single compiler call it can generate zero or more entries.

        :param execution:   executed command and working directory
        :param tools:       helper object to detect compiler
        :return: stream of CompilationDbEntry objects """

        candidate = cls._split_command(execution.cmd, tools)
        for source in candidate.files if candidate else []:
            output = candidate.output[0] if candidate.output else None
            phase = candidate.phase[0] if candidate.phase else '-c'
            result = Compilation(directory=execution.cwd,
                                 source=source,
                                 compiler=candidate.compiler,
                                 language=candidate.language,
                                 phase=phase,
                                 flags=candidate.flags,
                                 output=output)
            if os.path.isfile(result.source):
                yield result

    @classmethod
    def _split_compiler(cls, command, tools):
        """ A predicate to decide whether the command is a compiler call.

        :param command: the command to classify
        :param tools:   helper object to detect compiler
        :return: None if the command is not a compilation, or a tuple
                (compiler, language, rest of the command) otherwise """

        if command:  # not empty list will allow to index '0' and '1:'
            executable = os.path.basename(command[0])  # type: str
            parameters = command[1:]  # type: List[str]
            # 'wrapper' 'parameters' and
            # 'wrapper' 'compiler' 'parameters' are valid.
            # Additionally, a wrapper can wrap another wrapper.
            if tools.is_wrapper(executable):
                result = cls._split_compiler(parameters, tools)
                # Compiler wrapper without compiler is a 'C' compiler.
                return result if result else (command[0], C_LANG, parameters)
            # MPI compiler wrappers add extra parameters
            elif tools.is_mpi_wrapper(executable):
                # Pass the executable with full path to avoid pick different
                # executable from PATH.
                mpi_call = get_mpi_call(command[0])  # type: List[str]
                return cls._split_compiler(mpi_call + parameters, tools)
            # and 'compiler' 'parameters' is valid.
            elif tools.is_c_compiler(executable):
                return command[0], C_LANG, parameters
            elif tools.is_cxx_compiler(executable):
                return command[0], CPLUSPLUS_LANG, parameters
            elif tools.is_fortran_compiler(executable):
                return command[0], FORTRAN_LANG, parameters
        return None

    @classmethod
    def _split_command(cls, command, tools):
        """ Returns a value when the command is a compilation, None otherwise.

        :param command: the command to classify
        :param tools:   helper object to detect compiler
        :return: stream of CompilationCommand objects """

        logging.debug('input was: %s', command)
        # quit right now, if the program was not a C/C++ compiler
        compiler_and_arguments = cls._split_compiler(command, tools)
        if compiler_and_arguments is None:
            return None

        # the result of this method
        result = CompilationCommand(compiler=compiler_and_arguments[0],
                                    language=compiler_and_arguments[1],
                                    phase=[],
                                    flags=[],
                                    files=[],
                                    output=[])
        # iterate on the compile options
        args = iter(compiler_and_arguments[2])
        for arg in args:
            # quit when compilation pass is not involved
            if arg in {'-E', '-cc1', '-cc1as', '-M', '-MM', '-###'}:
                return None
            elif arg in {'-S', '-c'}:
                result.phase.append(arg)
            # ignore some flags
            elif arg in IGNORED_FLAGS:
                count = IGNORED_FLAGS[arg]
                for _ in range(count):
                    next(args)
            elif re.match(r'^-(l|L|Wl,).+', arg):
                pass
            # some parameters look like a filename, take those explicitly
            elif arg in {'-D', '-U', '-I', '-include'}:
                result.flags.extend([arg, next(args)])
            # get the output file separately
            elif arg == '-o':
                result.output.append(next(args))
            # parameter which looks source file is taken...
            elif re.match(r'^[^-].+', arg) and classify_source(arg):
                result.files.append(arg)
            # and consider everything else as compile option.
            else:
                result.flags.append(arg)
        logging.debug('output is: %s', result)
        # do extra check on number of source files
        return result if result.files else None


class CompilationDatabase:
    """ Compilation Database persistence methods. """

    @staticmethod
    def save(iterator, filename, field_output):
        # type: (Iterable[Compilation], str, bool) -> None
        """ Saves compilations to given file.

        :param filename: the destination file name
        :param iterator: iterator of Compilation objects. """

        entries = [entry.as_db_entry(field_output) for entry in iterator]
        with open(filename, 'w') as handle:
            json.dump(entries, handle, sort_keys=True, indent=4)

    @staticmethod
    def load(filename, tools):
        # type: (str, Tools) -> Iterable[Compilation]
        """ Load compilations from file.

        :param filename: the file to read from
        :param tools: helper object to detect compiler
        :returns: iterator of Compilation objects. """

        with open(filename, 'r') as handle:
            for entry in json.load(handle):
                for compilation in Compilation.from_db_entry(entry, tools):
                    yield compilation


def classify_source(filename, c_compiler=True):
    # type: (str, bool) -> str
    """ Classify source file names and returns the presumed language,
    based on the file name extension.

    :param filename:    the source file name
    :param c_compiler:  indicate that the compiler is a C compiler,
    :return: the language from file name extension. """

    mapping = {
        '.c': 'c' if c_compiler else 'c++',
        '.i': 'c-cpp-output' if c_compiler else 'c++-cpp-output',
        '.ii': 'c++-cpp-output',
        '.m': 'objective-c',
        '.mi': 'objective-c-cpp-output',
        '.mm': 'objective-c++',
        '.mii': 'objective-c++-cpp-output',
        '.C': 'c++',
        '.cc': 'c++',
        '.CC': 'c++',
        '.cp': 'c++',
        '.cpp': 'c++',
        '.cxx': 'c++',
        '.c++': 'c++',
        '.C++': 'c++',
        '.txx': 'c++',
        '.s': 'assembly',
        '.S': 'assembly',
        '.sx': 'assembly',
        '.asm': 'assembly',
        '.f95': 'fortran',
        '.F95': 'fortran',
        '.f90': 'fortran',
        '.F90': 'fortran',
        '.f': 'fortran',
        '.F': 'fortran',
        '.FOR': 'fortran',
        '.f77': 'fortran',
        '.fc': 'fortran',
        '.for': 'fortran',
        '.ftn': 'fortran',
        '.fpp': 'fortran'
    }

    __, extension = os.path.splitext(os.path.basename(filename))
    return mapping.get(extension)


def get_mpi_call(wrapper):
    # type: (str) -> List[str]
    """ Provide information on how the underlying compiler would have been
    invoked without the MPI compiler wrapper. """

    for query_flags in [['-show'], ['--showme']]:
        try:
            output = run_command([wrapper] + query_flags)
            if output:
                return shell_split(output[0])
        except Exception:
            pass
    # Fail loud
    raise RuntimeError("Could not determinate MPI flags.")


@contextlib.contextmanager
def temporary_directory(**kwargs):
    name = tempfile.mkdtemp(**kwargs)
    try:
        yield name
    finally:
        shutil.rmtree(name)


if __name__ == "__main__":
    sys.exit(intercept_build())
