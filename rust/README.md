# What's this?

This is a rust rewrite of the current master branch of this project.

# Why?

The current master branch is written in C++ and is not very well written.
I want to rewrite it in rust to make it more maintainable and easier to work with.

## What's wrong with the current codebase?

- The idea of disabling exception handling and using Rust-like result values is sound,
  but the implementation could be improved.
- The use of CMake as a build tool has caused several issues,
  including poor handling of third-party libraries and subprojects.
- Some dependencies are problematic:
  - Not all of them are available on all platforms.
  - Updating them can be challenging.

## What are the benefits of rewriting the project in Rust?

- Easy porting of the project to other platforms, including Windows
- Improved maintainability through the use of third-party libraries
  and better development tooling

# How?

The `3.x` version will be the last version of the C++ codebase.
The `4.x` version will be the first version of the rust codebase.

The `master` branch will be kept as the main release branch.
And the rust codebase will be developed on the `master` branch,
but it will be kept in a separate directory.

# When?

I will work on this project in my free time (as before).
