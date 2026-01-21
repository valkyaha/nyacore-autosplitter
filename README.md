# NYACore Autosplitter

Automatic boss defeat detection plugin for [NYA Core](https://github.com/valkyaha/HitCounter).

## Supported Games

- Dark Souls Remastered
- Dark Souls II: Scholar of the First Sin
- Dark Souls III
- Elden Ring
- Sekiro: Shadows Die Twice
- Armored Core VI

## Installation

1. Download `nyacore_autosplitter.dll` from the [latest release](https://github.com/valkyaha/nyacore-autosplitter/releases/latest)
2. Place it in your plugins folder:
   - Windows: `%APPDATA%/NYA Core/plugins/`
3. Restart NYA Core

## Requirements

- Windows 10/11
- NYA Core 3.1.0 or later

## How It Works

The autosplitter reads game memory to detect boss defeats automatically. It uses the same techniques as other speedrunning tools like LiveSplit and SoulSplitter.

**Note:** Because this tool reads game memory, some antivirus software may flag it as a false positive. This is expected behavior for tools of this type.

## Building from Source

```bash
cargo build --release
```

The DLL will be located at `target/release/nyacore_autosplitter.dll`.

## License

MIT License - see [LICENSE](LICENSE) for details.
