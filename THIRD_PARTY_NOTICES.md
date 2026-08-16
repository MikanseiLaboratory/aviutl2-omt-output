# Third-Party Notices

This file lists third-party software included with AviUtl2 OMT Live Output.
Crate versions are pinned by `Cargo.lock`. Regenerating the crate inventory:

```text
cargo about generate --workspace
```

`cargo-about` does not include the AviUtl ExEdit2 Plugin SDK headers consumed by `aviutl2-sys`. That license is included manually below.

## AviUtl ExEdit2 Plugin SDK

```text
---------------------------------
AviUtl ExEdit2 Plugin SDK License
---------------------------------

The MIT License

Copyright (c) 2025 Kenkun

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
```

## openmediatransport-rs

Pinned git revision: `55ffd08ab899f8017056886157fb0d130ab36d5c`

MIT License. Copyright (c) 2026 Open Media Transport Contributors and MikanseiLaboratory.

## vmx-rs

Transitive dependency of `openmediatransport-rs`, pinned by `Cargo.lock`.

MIT License. Copyright (c) 2026 Open Media Transport Contributors and MikanseiLaboratory.

## aviutl2-rs crates

`aviutl2` 0.43.0, `aviutl2-eframe` 0.43.0, and related crates (`aviutl2-sys`, and so on).

MIT License. Copyright (c) sevenc-nanashi / aviutl2-rs contributors.

## Remaining crates

See `Cargo.lock` for the complete, version-pinned dependency graph. Additional notices generated from that lockfile are appended after `cargo about generate`.

## Crate inventory (from Cargo.lock)

The following crate names are pinned in `Cargo.lock`. License texts for crates.io packages can be regenerated with `cargo about generate` when `cargo-about` is available.

ab_glyph, ab_glyph_rasterizer, accesskit, adler2, ahash, aho-corasick, android-activity, android-properties, anyhow, arboard, arrayref, arrayvec, as-raw-xcb-connection, atomic-waker, autocfg, aviutl2, aviutl2-alias, aviutl2-eframe, aviutl2-macros, aviutl2-omt-output, aviutl2-sys, base64, bitflags, block2, bumpalo, bytemuck, bytemuck_derive, byteorder-lite, bytes, calloop, calloop-wayland-source, cc, cfg_aliases, cfg-if, cgl, clipboard-win, color, combine, comptime-if, concurrent-queue, core_maths, core-foundation, core-foundation-sys, core-graphics, core-graphics-types, crc32fast, crossbeam-deque, crossbeam-epoch, crossbeam-utils, crunchy, cursor-icon, dashmap, decimal-rs, dispatch, dispatch2, displaydoc, dlib, document-features, downcast-rs, dpi, duplicate, ecolor, eframe, egui, egui_glow, egui-winit, either, emath, enumn, env_filter, epaint, epaint_default_fonts, equivalent, errno, error-code, euclid, fastrand, fax, fdeflate, fearless_simd, find-msvc-tools, flate2, flume, foldhash, font-types, fontconfig-parser, fontdb, foreign-types, foreign-types-macros, foreign-types-shared, form_urlencoded, futures-core, futures-sink, futures-task, futures-util, gethostname, getrandom, gl_generator, glifo, glow, glutin, glutin_egl_sys, glutin_wgl_sys, glutin-winit, guillotiere, half, harfrust, hashbrown, heck, hermit-abi, home, icu_collections, icu_locale_core, icu_normalizer, icu_normalizer_data, icu_properties, icu_properties_data, icu_provider, idna, idna_adapter, if-addrs, image, indexmap, itertools, jni, jni-macros, jni-sys, jni-sys-macros, jobserver, js-sys, khronos_api, kurbo, lazy_static, libc, libloading, libm, libredox, linebender_resource_handle, linux-raw-sys, litemap, litrs, lock_api, log, matchers, mdns-sd, memchr, memmap2, memoffset, miniz_oxide, mio, moxcms, ndk, ndk-context, ndk-sys, nohash-hasher, nu-ansi-term, num_enum, num_enum_derive, num-bigint, num-integer, num-rational, num-traits, objc-sys, objc2, objc2-app-kit, objc2-cloud-kit, objc2-contacts, objc2-core-data, objc2-core-foundation, objc2-core-graphics, objc2-core-image, objc2-core-location, objc2-encode, objc2-foundation, objc2-io-surface, objc2-link-presentation, objc2-metal, objc2-quartz-core, objc2-symbols, objc2-ui-kit, objc2-uniform-type-identifiers, objc2-user-notifications, once_cell, openmediatransport, orbclient, owned_ttf_parser, parking_lot, parking_lot_core, pastey, peniko, percent-encoding, pin-project, pin-project-internal, pin-project-lite, pkg-config, plain, png, polling, polycool, potential_utf, proc-macro-crate, proc-macro2, proc-macro2-diagnostics, process_path, profiling, pxfm, quick-error, quick-xml, quote, r-efi, raw-window-handle, rayon, rayon-core, read-fonts, redox_syscall, regex, regex-automata, regex-syntax, rmp, rmp-serde, ron, roxmltree, rustc_version, rustix, rustversion, ruzstd, same-file, scoped-tls, scopeguard, sctk-adwaita, self_cell, semver, serde, serde_core, serde_derive, sharded-slab, shlex, simd_cesu8, simd-adler32, simdutf8, skrifa, slab, slotmap, smallvec, smithay-client-toolkit, smithay-clipboard, smol_str, socket-pktinfo, socket2, spin, stable_deref_trait, stack-buf, static_assertions, strict-num, strum, strum_macros, syn, synstructure, thiserror, thiserror-impl, thread_local, tiff, tiny-skia, tiny-skia-path, tinystr, tinyvec, tinyvec_macros, toml_datetime, toml_edit, toml_parser, tracing, tracing-attributes, tracing-core, tracing-log, tracing-subscriber, ttf-parser, twox-hash, typeid, unicode-general-category, unicode-ident, unicode-segmentation, url, utf8_iter, uuid, valuable, vello_common, vello_cpu, version_check, vmx, walkdir, wasi, wasip2, wasm-bindgen, wasm-bindgen-futures, wasm-bindgen-macro, wasm-bindgen-macro-support, wasm-bindgen-shared, wayland-backend, wayland-client, wayland-csd-frame, wayland-cursor, wayland-protocols, wayland-protocols-experimental, wayland-protocols-misc, wayland-protocols-plasma, wayland-protocols-wlr, wayland-scanner, wayland-sys, web-sys, web-time, webbrowser, weezl, winapi, winapi-i686-pc-windows-gnu, winapi-util, winapi-x86_64-pc-windows-gnu, windows, windows_aarch64_gnullvm, windows_aarch64_msvc, windows_i686_gnu, windows_i686_gnullvm, windows_i686_msvc, windows_x86_64_gnu, windows_x86_64_gnullvm, windows_x86_64_msvc, windows-collections, windows-core, windows-future, windows-implement, windows-interface, windows-link, windows-numerics, windows-result, windows-strings, windows-sys, windows-targets, windows-threading, winit, winnow, wit-bindgen, writeable, x11-dl, x11rb, x11rb-protocol, xcursor, xkbcommon-dl, xkeysym, xml-rs, yoke, yoke-derive, zerocopy, zerocopy-derive, zerofrom, zerofrom-derive, zerotrie, zerovec, zerovec-derive, zlib-rs, zune-core, zune-jpeg
