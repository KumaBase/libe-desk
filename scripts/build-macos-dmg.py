"""Build a signed drag-to-install DMG from an already notarized app.

Requires macOS, dmgbuild and Pillow. The resulting DMG still needs notarization.
"""

import argparse
import json
from pathlib import Path
import subprocess
import tempfile

import dmgbuild
from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
app = ROOT / "src-tauri/target/release/bundle/macos/Libe Desk.app"
output = args.output.resolve()
if output.exists():
    parser.error(f"Output already exists: {output}")

subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
subprocess.run(["xcrun", "stapler", "validate", str(app)], check=True)
arch = subprocess.check_output(
    ["/usr/bin/lipo", "-archs", str(app / "Contents/MacOS/libe-desk")], text=True
).strip()
if arch != "arm64":
    parser.error(f"This installer layout is labeled Apple Silicon; found {arch}")
config = json.loads((ROOT / "src-tauri/tauri.signed.conf.json").read_text())
identity = config["bundle"]["macOS"]["signingIdentity"]
output.parent.mkdir(parents=True, exist_ok=True)

with tempfile.TemporaryDirectory(prefix="libe-desk-dmg-") as work:
    background = Path(work) / "background.png"
    canvas = Image.new("RGB", (660, 400), "#f5f7fb")
    draw = ImageDraw.Draw(canvas)
    font_path = "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc"
    title = ImageFont.truetype(font_path, 28)
    body = ImageFont.truetype(font_path, 17)
    small = ImageFont.truetype(font_path, 12)
    draw.text((330, 42), "Libe Desk", font=title, fill="#172640", anchor="mt")
    draw.text((330, 89), "アプリケーションへドラッグしてインストール", font=body,
              fill="#465670", anchor="mt")
    draw.line((288, 208, 372, 208), fill="#527bc0", width=5)
    draw.line((355, 191, 372, 208, 355, 225), fill="#527bc0", width=5)
    draw.text((330, 331), "Apple Silicon Mac 用", font=small, fill="#6b7890", anchor="mt")
    canvas.save(background)
    dmgbuild.build_dmg(str(output), "Libe Desk", settings={
        "format": "UDZO",
        "files": [str(app)],
        "symlinks": {"Applications": "/Applications"},
        "background": str(background),
        "icon": str(ROOT / "src-tauri/icons/icon.icns"),
        "window_rect": ((180, 140), (660, 400)),
        "icon_locations": {"Libe Desk.app": (170, 208), "Applications": (490, 208)},
        "icon_size": 96,
        "text_size": 14,
        "default_view": "icon-view",
        "show_toolbar": False,
        "show_status_bar": False,
        "show_sidebar": False,
    })

subprocess.run(["codesign", "--sign", identity, "--timestamp", str(output)], check=True)
subprocess.run(["codesign", "--verify", "--strict", str(output)], check=True)
print(f"Created signed DMG (notarization still required): {output}")
