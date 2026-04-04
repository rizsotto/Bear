## Man page

`bear.1.md` is the source. `bear.1` is generated from it.

## Generating the man page

After editing `bear.1.md`, regenerate:

```bash
pandoc -s -t man bear.1.md -o bear.1
```

## Rules

- Keep `bear.1.md` and `bear.1` in sync -- always regenerate after edits
- Format follows pandoc man page conventions (% title, % author, % date header)
- CLI flags and options must match what `bear/src/args.rs` defines via `clap`
- If CLI behavior changes, update `bear.1.md` first, then regenerate
