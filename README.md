# mtime-rewind

Rewind the `mtime` attribute of files whose modification advanced since the last execution without a content change.

This is useful to avoid unnecessary cache invalidations in systems using `mtime` to detect changes, for example [Rust's cargo](https://doc.rust-lang.org/cargo/) (see [this issue](https://github.com/rust-lang/cargo/issues/6529)).

More generally, see the [`mtime` comparison considered harmful blog post](https://apenwarr.ca/log/20181113).

## Usage

```console
$ mtime-rewind
Rewind the mtime of files whose mtime advanced since the last execution without a content change

Usage: mtime-rewind [OPTIONS] <ROOT>

Arguments:
  <ROOT>

Options:
      --dry   Do not edit only mtime, only list the changes that would be made
  -h, --help  Print help

```

- The first execution will store hashes and modification times of files in a `.hashprint` file at the root. Hidden files and [cache directories](https://bford.info/cachedir/) are ignored.
- Subsequent executions will rewind files that have not changed to the previous modification time, and update the modification times of other files if necessary.

Typically, `mtime-rewind` can be executed as the first step of a CI build.

## Example

```console
$ mtime-rewind ~/project
[INFO mtime_rewind] Computing hashes...
[INFO mtime_rewind] Computed hashes for 9 files
[INFO mtime_rewind] Writing hashes for the first time...
[INFO mtime_rewind] Wrote "/root/project/.hashprint"
[INFO mtime_rewind] Done
$ touch src/main.rs
[INFO mtime_rewind] Computing hashes...
[INFO mtime_rewind] Computed hashes for 9 files
[INFO mtime_rewind] Restoring modification times for unchanged files...
[INFO mtime_rewind] Loading cached state...
[INFO mtime_rewind] Loaded hashes for 9 files
[INFO mtime_rewind] Rewinding "/root/project/src/main.rs" from SystemTime { tv_sec: 1693727396, tv_nsec: 146042169 } to SystemTime { tv_sec: 1693727019, tv_nsec: 668108072 } as its contents did not change
[INFO mtime_rewind] 1 files rewinded
[INFO mtime_rewind] Saving the new state...
[INFO mtime_rewind] Wrote "/root/project/.hashprint"
[INFO mtime_rewind] Done
```
