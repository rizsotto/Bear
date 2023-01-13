% BEAR-CITNAMES(1) Bear User Manuals
% László Nagy
% Jan 02, 2023

# NAME

bear-citnames - deduce command semantic

# SYNOPSIS

bear citnames [*options*] \--input \<file\> \--output \<file\>

# DESCRIPTION

The name citnames comes from to reverse the word "semantic".

Because when you type a command, you know your intent. The command
execution is just a thing to achieve your goal. This program takes
the command which was executed, and try to find out what the intent
was to run that command. It deduces the semantic of the command.

This is useful to generate a compilation database. Citnames get a
list of commands, and it creates a JSON compilation database. (This
is currently the only output of the tool.)

# OPTIONS

\--version
:   Print version number.

\--help
:   Print help message.

\--verbose
:   Enable verbose logging.

\--input *file*
:   Specify input file. (Default file name provided.) The input is a
    command execution list, with some extra information. The syntax
    is detailed in a separate section.

\--output *file*
:   Specify output file. (Default file name provided.) The output is
    currently a JSON compilation database.

\--append
:   Use previously generated output file and append the new entries to it.
	This way you can run continuously during work, and it keeps the
	compilation database up to date. File deletion and addition are both
	considered. But build process change (compiler flags change) might
	cause duplicate entries.

\--run-checks
:   Allow the program to verify file location checks on the current machine
    it runs. (Default value provided. Run help to query it.) This is important
    if the execution list is not from the current host.

\--config *file*
:   Specify a configuration file. The configuration file captures how
    the output should be formatted and which entries it shall contain.

# EXIT STATUS

Citnames exit status is non-zero in case of IO problems, otherwise it's zero.
The exit status is independent of how many command it recognized or was
it recognized at all.

# OBSERVABILITY

Any insight about the command recognition logic can be observed with `--verbose`
flag on the standard error. Otherwise, the command is silent.

# INPUT FILE

It's a JSON file, with the command execution history. (Plus some metadata, that
is useful for debugging the application which was produced it.) This file can
be produced by the `bear intercept` command, which records the process executions
of a build.

Read more about the syntax of the file in the `bear-intercept(1)` man page.

# OUTPUT FILE

Currently, the only output format is the JSON compilation database.
Read more about the syntax of that in the `bear(1)` man page. 

# CONFIG FILE

The config file influences the command recognition (by the section "compilation")
and the output format (by the section "output").

The config file is optional. The program will use default values, which can be
dumped with the `--verbose` flags.

Some parts of the file has overlap with the command line arguments. If both present
the command line argument overrides the config file values.

```json
{
  "compilation": {
    "compilers_to_recognize": [
      {
        "executable": "/usr/bin/mpicc",
        "flags_to_add": ["-I/opt/MPI/include"],
        "flags_to_remove": ["-Wall"]
      }
    ],
    "compilers_to_exclude": []
  },
  "output": {
    "content": {
      "include_only_existing_source": true,
      "paths_to_include": [],
      "paths_to_exclude": [],
      "duplicate_filter_fields": "file_output"
    },
    "format": {
      "command_as_array": true,
      "drop_output_field": false
    }
  }
}
```

`compilation.compilers_to_recognize`
:   where compiler can be specified, which are not yet recognized by default.
    The `executable` is an absolute path to the compiler. The `flags_to_add`
    is an optional attribute, which contains flags which will append to the final
    output. (It's a good candidate to use this for adding OpenMPI compiler wrapper
    flags from the `mpicc --showme:compile` output.) The `flags_to_remove` is
    an optional attribute, where the given flags will be removed for the final
    argument list. (The flags checked for equality only, no regex match. Flags
    with arguments are not good candidates to put here, because the removal logic
    is too simple for that.)

`compilation.compilers_to_exclude`
:   this is an optional list of executables (with absolute path) which needs to
    be removed from the output.

`output.content`
:   The `paths_to_include` and `paths_to_exclude` are for filter out entries from
    these directories. (Directory names can be absolute paths or relative to the
    current working directory if the `--run-checks` flag passed.)
    The `include_only_existing_source` allows or disables file check for the output.
    The `--run-checks` flag overrides this config value. The `duplicate_filter_fields`
    select the method how duplicate entries are detected in the output. The possible
    values for this field are: `all`, `file` and `file_output`.

`output.format`
:   The `command_as_array` controls which command field is emitted in the output.
    True produces `arguments`, false produces `command` field. The `drop_output_field`
    will disable the `output` field from the output.

# SEE ALSO

`bear(1)`, `bear-intercept(1)`

# COPYRIGHT

Copyright (C) 2012-2022 by László Nagy
<https://github.com/rizsotto/Bear>
