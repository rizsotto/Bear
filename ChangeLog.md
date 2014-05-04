## 1.4.1

Bugfixes:

  - fix output entries have c14n file path (#61, @nickhutchinson)
  - fix error message on missing config file (#60, @viraptor)


## 1.4 (2014-01-12)

Bugfixes:

  - fix typo in the README.md (#48, @breser)
  - fix typo in the man page (#49, @sebastinas)
  - fix cmake file to honor given CFLAGS (#50, @sebastinas)
  - fix execle causes segfault on 32 bit systems (#51, #52, @breser, @sebastinas)


## 1.3 (2013-12-18)

Features:

  - set empty cancel parameter list as default (#39, #43)
  - implement verbose filter message at the end of the run (#41)

Bugfixes:

  - fix process stops when ctrl-z pressed (#40, @bbannier)
  - fix non filtered output option renamed from debug (#44, @mikemccracken)
  - fix broken build on OS X (#46, @breser)
  - fix documentation (@mlq)
  - fix posix_spawn* call not implemented (#43, @agentsim, @apoluektov)


## 1.2 (2013-10-01)

Features:

  - dependency file generation compiler calls are _optionally_ filtered (#35, @lonico)
  - use config file for compiler call filtering parameters (#38, @lonico, @peti)

Bugfixes:

  - fix end-to-end test on OS X (#37, @smmckay)
  - fix memory leaks detected by static analyser


## 1.1 (2013-08-01)

Features:

  - dependency file generation compiler calls `-M` are filtered (#35, @chrta)
  - smaller memory footprint (less allocation, code went for places when it is called)
  - add version query to command line

Bugfixes:

  - fix memory leaks detected by static analyser


## 1.0 (2013-06-27)

Features:

  - change license to GPLv3

Bugfixes:

  - fix process syncronization problem (#33, @blowback)
  - fix malloc/realloc usage (#34, @mlq)


## 0.5 (2013-06-09)

Features:

  - use temporary directory for default socket (#29, @sebastinas)

Bugfixes:

  - fix bashism in test (#27, @sebastinas)
  - fix temporary socket dir problem introduce by new code (#31, @lukedirtwalker)
  - fix bug introduced by report filtering (#32, @lukedirtwalker)


## 0.4 (2013-04-26)

Features:

  - man page generation is optional (#18, @Sarcasm)
  - port to OS X (#24, @breser)

Bugfixes:

  - fix json output on whitespaces (#19)
  - fix socket reading problem (#20, @brucestephens)
  - improved signal handling (#21)
  - build system checks for available `exec` functions (#22)


## 0.3 (2013-01-09)

Features:

  - query known compilers which are play roles in filtering (#10)
  - query recognised source file extensions which are filtering (#11)
  - man page added (#12)
  - pacage generation target added to `cmake` (#15)

Bugfixes:

  - fix child process termination problem
  - test added: build result propagation check


## 0.2 (2013-01-01)

Features:

  - add debug output

Bugfixes:

  - test added: unit test, end-to-end test and full `exec` family coverage (#4)
  - `scons` does pass empty environment to child processes (#9)
  - fix `execle` overriding bug (#13)
  - fix json output (#14)


## 0.1 (2012-11-17)

Features:

  - first working version
  - [Travis CI](https://travis-ci.org/rizsotto/Bear) hook set up
