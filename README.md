# The semver trick

**The semver trick refers to publishing a breaking change to a Rust library
without requiring a coordinated upgrade across its downstream dependency
graph.** The trick is built around having one version of your library declare a
dependency on a newer version of the same library.

<br>

## Illustrative example

The Rust library ecosystem has a history of traumatic library upgrades. The
upgrade of [`libc`] from 0.1 to 0.2 is known as the "libcpocalypse". Another
frequent culprit was pre-1.0 [Serde], with the upgrades from 0.7 to 0.8 to 0.9
to 1.0 requiring ecosystem-wide effort.

[`libc`]: https://crates.io/crates/libc
[Serde]: https://serde.rs/

The cause of the difficulty was the large number of crates using types from
these libraries in their public API.

By way of example, consider a simplified version of the `libc` crate that
exposes only two things: the `c_void` type and the `EVFILT_AIO` constant from
NetBSD.

```rust
// libc 0.2.0

pub type c_void = /* it's complicated */;

pub const EVFILT_AIO: i32 = 2;
```

The `c_void` type becomes widely used as hundreds of libraries want to expose
functions that are ABI-compatible with C's `void *` type. Meanwhile the
`EVFILT_AIO` constant is less commonly used and never in the public API of
downstream crates.

```rust
extern "C" {
    // Usable from C as:
    //
    //    void qsort(
    //        void *base,
    //        size_t nitems,
    //        size_t size,
    //        int (*compar)(const void *, const void*));
    //
    // The `c_void` type is now part of the public API of this crate.
    pub fn qsort(
        base: *mut c_void,
        nitems: usize,
        size: usize,
        compar: Option<unsafe extern fn(*const c_void, *const c_void) -> c_int>,
    );
}
```

After some time, it is discovered that `EVFILT_AIO` should have been defined as
`uint32_t` rather than `int32_t` to match how it is used elsewhere in NetBSD
header files ([rust-lang/libc#506]).

[rust-lang/libc#506]: https://github.com/rust-lang/libc/pull/506

This fix would be a breaking change to the `libc` crate. Existing code that
passes `libc::EVFILT_AIO` to a function accepting an argument of type `int32_t`
would be broken, and this needs to be reflected in the semver version of the
`libc` crate.

Here is where things go wrong.

<br>

## Coordinated upgrades

Suppose we make the fix and publish it as a breaking change.

```rust
// libc 0.3.0

pub type c_void = /* it's complicated */;

pub const EVFILT_AIO: u32 = 2;
```

Despite the fact that the definition of `c_void` has not changed, technically
the `c_void` from libc 0.2 and the `c_void` from libc 0.3 are different types.
In Rust (as in C, for that matter), two structs are not interchangeable just
because they have the same fields; passing one to a function that is declared to
take the other is a compile error.

That means if crate A depends on crate B which depends on `libc`, and B uses
`c_void` in the public API of some function called by A, then A cannot upgrade
to libc 0.3 until B has upgraded to libc 0.3. If A upgrades before B, then A is
going to try to pass libc 0.3's `c_void` to B's function that still expects libc
0.2's `c_void` and will not compile.

What needs to happen is first B upgrades to libc 0.3, releases this as a major
version bump of B (because its public API has changed in a breaking way), and
then A may upgrade to the new version of B.

For longer dependency chains this is a huge ordeal and requires coordinated
effort across dozens of developers. During the most recent libcpocalypse, Servo
found themselves coordinating an upgrade of 52 libraries over a period of three
months ([servo/servo#8608]).

[servo/servo#8608]: https://github.com/servo/servo/issues/8608

<br>

## The trick

At the heart of the problem is having a widely used API caught up in the
breakage of a much less widely used API. Rust and Cargo are capable of handling
this predicament in a better way.

All we need is one modification to the `c_void` / `EVFILT_AIO` example from
above.

After making the breaking change and publishing it as libc 0.3.0, we release one
final minor version of the 0.2 series and re-export the unchanged API(s) from
0.3.

In Cargo.toml:

```toml
[package]
name = "libc"
version = "0.2.1"

[dependencies]
libc = "0.3"  # future version of itself
```

And in lib.rs:

```rust
// libc 0.2.1

pub use libc::c_void;  // reexport from libc 0.3, as per Cargo.toml

pub const EVFILT_AIO: i32 = 2;
```

This way we avoid the problem of having two `c_void` types that look the same
but are not interchangeable. Here the `c_void` from libc 0.2.1 and the `c_void`
from libc 0.3.0 are precisely the same type.

The libcpocalypse scenario is averted because users of `libc` can upgrade from
0.2 to 0.3 at their leisure, in any order, without needing to bump their own
semver major version.

<br>

## Advanced trickery

With some care and creativity, the technique above can be generalized to lots of
different breaking change situations. The `semver-trick` example crate included
in this repo demonstrates some types of changes that can be accomodated.

- [`semver_trick::Unchanged`] is interchangeable across 0.2 and 0.3.
- [`semver_trick::Removed`] exists in 0.2 but not 0.3.
- [`semver_trick::Added`] exists in 0.3 but not 0.2.
- [`semver_trick::before::Moved`] has been moved to [`semver_trick::after::Moved`].

[`semver_trick::Unchanged`]: https://docs.rs/semver-trick/0.2.0/semver_trick/struct.Unchanged.html
[`semver_trick::Removed`]: https://docs.rs/semver-trick/0.2.0/semver_trick/struct.Removed.html
[`semver_trick::Added`]: https://docs.rs/semver-trick/0.3.0/semver_trick/struct.Added.html
[`semver_trick::before::Moved`]: https://docs.rs/semver-trick/0.2.0/semver_trick/before/struct.Moved.html
[`semver_trick::after::Moved`]: https://docs.rs/semver-trick/0.3.0/semver_trick/after/struct.Moved.html

<br>

## Limitations

This is not the silver bullet that solves all occurrences of dependency hell.

Fundamentally the semver trick is beneficial when a crate needs to break a
rarely used API while leaving widely used APIs unchanged, or when a crate wants
to shuffle types around in its module hierarchy.

Most other types of breakage are not helped by this trick, including the
following concrete examples:

- Adding a new method to a widely used trait that is not [sealed],
- Bumping a major version of a public dependency that is not itself using the
  semver trick,
- Raising the minimum supported version of rustc.

[sealed]: https://rust-lang-nursery.github.io/api-guidelines/future-proofing.html#c-sealed

<br>

## Other tricks

Where the semver trick is not applicable, it can be possible to mitigate the
impact of breaking changes in other ways.

- The [Serde legacy shims] demonstrate a technique for allowing downstream
  libraries to provide trait impls simultaneously across multiple incompatible
  versions of a library.
- The [Future Proofing] chapter of the Rust API guidelines gives some
  suggestions for designing APIs that do not require breaking changes in the
  first place.

[Serde legacy shims]: https://github.com/serde-rs/legacy
[Future Proofing]: https://rust-lang-nursery.github.io/api-guidelines/future-proofing.html

<br>

#### License

<sup>
To the extent that it constitutes copyrightable work, the idea of depending on a
future version of the same library is licensed under the
CC0 1.0 Universal license (<a href="LICENSE-CC0">LICENSE-CC0</a>)
and may be used without attribution. This document and the accompanying
<code>semver-trick</code> example crate are licensed under either of
Apache License, Version 2.0 (<a href="LICENSE-APACHE">LICENSE-APACHE</a>)
or
MIT license (<a href="LICENSE-MIT">LICENSE-MIT</a>)
at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this codebase by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
</sub>
