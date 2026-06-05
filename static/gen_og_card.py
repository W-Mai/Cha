#!/usr/bin/env python3
"""Generate the social-preview card for Cha.

Embeds static/logo.svg (rendered at 320×320) as base64 inside an SVG
template, then renders SVG → 1200×628 PNG at static/og-card.png.

oranda copies the whole `static/` dir into the deployed site root,
so the image lands at https://cha.to01.icu/static/og-card.png — that's
the URL referenced by oranda.json's marketing.social.image and by
book/theme/head.hbs's og:image meta. No public/ copy is needed; oranda
clears public/ on every build.

Usage:
    cd cha repo root
    python3 static/gen_og_card.py

Requires:
    rsvg-convert (Homebrew: librsvg)
"""

import base64
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOGO_SVG = ROOT / "static" / "logo.svg"
LOGO_PNG_TMP = ROOT / "static" / ".og-logo-320.png"  # transient
OG_SVG = ROOT / "static" / "og-card.svg"
OG_PNG_OUT = ROOT / "static" / "og-card.png"

# 1. Render logo to 320×320 PNG so we don't need to re-process its
#    embedded path geometry inside the OG SVG.
subprocess.run(
    ["rsvg-convert", "-w", "320", "-h", "320",
     str(LOGO_SVG), "-o", str(LOGO_PNG_TMP)],
    check=True,
)
logo_b64 = base64.b64encode(LOGO_PNG_TMP.read_bytes()).decode("ascii")

# 2. Compose the OG card SVG.
SVG_TEMPLATE = """<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
     width="1200" height="628" viewBox="0 0 1200 628">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%"   stop-color="#fff7ed"/>
      <stop offset="55%"  stop-color="#fed7aa"/>
      <stop offset="100%" stop-color="#fb923c"/>
    </linearGradient>
    <filter id="cardShadow" x="-10%" y="-10%" width="120%" height="120%">
      <feDropShadow dx="0" dy="14" stdDeviation="20" flood-color="#7c2d12" flood-opacity="0.18"/>
    </filter>
  </defs>

  <rect width="1200" height="628" fill="url(#bg)"/>
  <circle cx="1080" cy="80"  r="180" fill="#ffffff" opacity="0.18"/>
  <circle cx="120"  cy="560" r="140" fill="#ffffff" opacity="0.15"/>

  <!-- Card -->
  <g filter="url(#cardShadow)">
    <rect x="60" y="80" width="1080" height="468" rx="32" ry="32" fill="#ffffff"/>
  </g>

  <!-- Logo, embedded as base64 PNG so SVG renders self-contained -->
  <image x="110" y="154" width="320" height="320"
         xlink:href="data:image/png;base64,__LOGO_B64__"/>

  <!-- Cha (display) -->
  <text x="470" y="270"
        font-family="-apple-system, BlinkMacSystemFont, 'SF Pro Display', 'Segoe UI', 'Helvetica Neue', Arial, sans-serif"
        font-size="120" font-weight="800" fill="#1f1206" letter-spacing="-3">Cha</text>

  <!-- 察 + sub-tagline -->
  <text x="470" y="350"
        font-family="-apple-system, 'PingFang SC', 'Hiragino Sans GB', 'Microsoft YaHei', 'Noto Sans CJK SC', sans-serif"
        font-size="48" font-weight="600" fill="#7c2d12">察 · Code Health Analyzer</text>

  <!-- Tagline (kept tight to fit inside the white card right edge at x=1140) -->
  <text x="470" y="410"
        font-family="-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Segoe UI', 'Helvetica Neue', Arial, sans-serif"
        font-size="22" font-weight="400" fill="#5a3a20">Pluggable code-smell detection · 34 detectors · WASM plugins</text>

  <!-- URL -->
  <text x="470" y="462"
        font-family="ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace"
        font-size="22" font-weight="500" fill="#9a3412">cha.to01.icu</text>
</svg>
"""

OG_SVG.write_text(SVG_TEMPLATE.replace("__LOGO_B64__", logo_b64), encoding="utf-8")

# 3. SVG → PNG.
subprocess.run(
    ["rsvg-convert", "-w", "1200", "-h", "628",
     str(OG_SVG), "-o", str(OG_PNG_OUT)],
    check=True,
)

# 4. Drop the transient render.
LOGO_PNG_TMP.unlink(missing_ok=True)

print(f"  → {OG_PNG_OUT.relative_to(ROOT)}: {OG_PNG_OUT.stat().st_size // 1024} KB")
