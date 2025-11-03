# Icon Support in MetaGit

MetaGit supports beautiful icons in the terminal output, with automatic detection for Nerd Fonts.

## Enabling Nerd Font Icons

To use Nerd Font icons instead of standard Unicode symbols, set the environment variable:

```bash
export NERD_FONT=1
# or
export USE_NERD_FONT=1
```

You can also enable it per-command:

```bash
NERD_FONT=1 mgit status
NERD_FONT=1 mgit run build_test
```

## Icon Sets

### Without Nerd Fonts (Default)
- Repository: ⚡
- Branch: ⎇
- Success: ✓
- Error: ✗
- Warning: ⚠
- Waiting: ⏳
- Running: ⚙

### With Nerd Fonts Enabled
When `NERD_FONT=1` is set, MetaGit displays proper Nerd Font glyphs:
- Repository:  (GitHub icon)
- Branch:  (Git branch icon)
- Success:  (Check circle)
- Error:  (Times circle)
- Warning:  (Exclamation triangle)
- Waiting:  (Clock icon)
- Running:  (Cog/gear icon)

## Installing Nerd Fonts

To get the best experience, install a Nerd Font:

1. Visit https://www.nerdfonts.com/
2. Download a font (recommended: JetBrainsMono Nerd Font, FiraCode Nerd Font, or Hack Nerd Font)
3. Install the font on your system
4. Configure your terminal to use the Nerd Font
5. Set `export NERD_FONT=1` in your shell profile (~/.bashrc, ~/.zshrc, etc.)

## Terminal Compatibility

The default Unicode icons work in all modern terminals. Nerd Font icons require:
- A Nerd Font installed and configured in your terminal
- A terminal that supports Unicode Private Use Area characters

Tested terminals:
- ✓ Alacritty (with Nerd Font)
- ✓ Kitty (with Nerd Font)
- ✓ iTerm2 (macOS, with Nerd Font)
- ✓ Windows Terminal (with Nerd Font)
- ✓ GNOME Terminal (with Nerd Font)
- ✓ Most modern terminals with proper font configuration

## Making it Permanent

Add to your `~/.bashrc` or `~/.zshrc`:

```bash
# Enable Nerd Font icons in mgit
export NERD_FONT=1
```

Then reload your shell:

```bash
source ~/.bashrc  # or ~/.zshrc
```
