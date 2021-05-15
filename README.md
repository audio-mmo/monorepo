# monorepo

The project ammo monorepo

This is an attempt to build an MMO engine optimized for 2D as a set of libraries, the end goal of which is hopefully a proper MMO for the blind.  Most of the pieces for this don't exist for one reason or another (e.g. Bevy isn't suitable because it's bad at scripting and scales in the wrong fashion) so we'll be bootstrapping mostly from scratch.

More info later.  At the moment [Synthizer](https://github.com/synthizer/synthizer) is still my primary project, but I needed a change of pace and this is the next thing, so I started it now.

Note that while this project uses the Boost license, most of Rust's ecosystem is Apache/BSD: if you depend on these components you will likely still need to collect the licenses of your dependencies.


## Components

Component | Description
--- | ---
`ammo_chunked_array` | A chunked, compressed array, primarily used to hold tilemaps.
