# Security policy

## Supported versions

Only the latest release gets security fixes. Older 4.x releases do not
get backports; upgrade to the latest release instead.

## Reporting a vulnerability

Please do not open a public issue for a security problem.

Report it privately through GitHub:
https://github.com/rizsotto/Bear/security/advisories/new

Include the Bear version, your platform, and the steps to reproduce.
You should get a first answer within 14 days. Once a fix is released,
the advisory is published and credits the reporter, unless you ask
otherwise.

## Scope

Bear runs your build and records the compiler calls it sees. It runs
with your privileges and trusts the build it is given. Issues in that
area that are in scope:

- the preload library (`libexec.so`) or the wrapper changing what the
  build executes,
- the driver and the reporting channel between it and the intercepted
  processes,
- the written `compile_commands.json` leaking or corrupting data
  beyond what the build itself passed to the compiler.

A build script that is already malicious is out of scope; Bear does not
sandbox the build.
