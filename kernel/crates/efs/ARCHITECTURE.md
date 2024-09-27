# Architecture

This document describes the high-level architecture of the crate efs.

## Bird's Eye View

On the highest level, efs is a crate that takes devices containing a filesystem and manipulates them through generic traits.

More specifically, a device is an object capable of storing a fixed-amount of contiguous `Copy` objects, usually `u8` (thus bytes). Moreover, it is possible to read and write at any point of a device directly with a given position named `Address` in this crate.

A filesystem designates here two things: a POSIX-compatible (and not compliant) hierarchical organization of files in an operating system, and a structure capable of managing it. More precisely, the filesystem object only acts like an entry-point for the file hierarchy: all the usual operations on files are handled by the file traits.

## Entry points

The `FileSystem` trait, in the `fs` module, defines the most important interface of the crate, the one to manipulate filesystems.

The `Device` trait, in the `dev` module, defines the interface to manipulate devices with slices of objects. The module also contains useful implementation of the trait for common structures.

`src/file.rs` contains all the traits to manipulate the different types of files that one can find in a POSIX filesystem. In particular, `File`, `Regular`, `Directory` and `SymbolicLink` are the one to look at at the beginning.

## Code Map

Only the important modules are described here.

### `file.rs`

This file contains all the basic traits for POSIX files' manipulation. In particular, regular files, directories and symbolic links have more methods than the other file types (block device, character device, named pipe and unix socket for the mandatory ones) because they do not need a virtual filesystem to be used.

All traits have read-only equivalent.

### `types.rs`

Definition of needed types that can be found in [the POSIX header `<sys/types.h>`](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/sys_types.h.html).

### `path.rs`

`no_std` implementation of POSIX paths. This module is very close to its `std` equivalent.

### `io.rs`

`no_std` implementation of basic I/O operations, namely `Read`, `Write` and `Seek`. Moreover, this module contains an interface to use this crate in a `std` environment.

### `dev`

This directory contains all the definitions linked to the device usage. In particular, the trait `Device` and its basic implementation are located here.

### `fs`

This directory contains all the definitions linked to the filesystems. In particular, the trait `FileSystem` and the implemented filesystems are located here.

### `fs/structures`

This directory contains structures that can be found on several filesystems, and that are common enough to be implemented with generalization in sight.

### `fs/<filesystem name>`

All the folders of the form `fs/<filesystem name>` are implementations of an actual filesystem. There is no indication on how to structure the content of those folders as it depends a lot on the filesystem definition.

## Code Invariants

### Filesystem state

In an execution where no error is produced, if a the filesystem starts in a coherent state, all calls to user-available functions (so public function) should leave the filesystem in a coherent state.

Moreover, in this context, all the functions leaving the filesystem in a incoherent state must be marked as `unsafe`, with a dedicated "Safety" section in the documentation.
