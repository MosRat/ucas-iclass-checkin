# /// script
# requires-python = ">=3.11"
# ///
"""Generate centered SVG brand assets from a shared geometric definition."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]

DARK = "#30261F"
ACCENT = "#A27E55"
TILE = "#F5EFE7"
TILE_STROKE = "#D7C5AE"


@dataclass(frozen=True)
class Point:
    x: float
    y: float


@dataclass(frozen=True)
class SymbolSpec:
    width: float
    height: float
    top_start: Point
    top_line_end: Point
    top_arc_end: Point
    top_vertical_end: Point
    bottom_start: Point
    bottom_line_end: Point
    bottom_arc_end: Point
    bottom_vertical_end: Point
    check_start: Point
    check_mid: Point
    check_end: Point
    arc_radius: float

    @property
    def points(self) -> tuple[Point, ...]:
        return (
            self.top_start,
            self.top_line_end,
            self.top_arc_end,
            self.top_vertical_end,
            self.bottom_start,
            self.bottom_line_end,
            self.bottom_arc_end,
            self.bottom_vertical_end,
            self.check_start,
            self.check_mid,
            self.check_end,
        )


BASE_SYMBOL = SymbolSpec(
    width=54,
    height=46,
    top_start=Point(0, 0),
    top_line_end=Point(32, 0),
    top_arc_end=Point(50, 18),
    top_vertical_end=Point(50, 24),
    bottom_start=Point(44, 46),
    bottom_line_end=Point(14, 46),
    bottom_arc_end=Point(-4, 28),
    bottom_vertical_end=Point(-4, 22),
    check_start=Point(7, 17),
    check_mid=Point(19, 29),
    check_end=Point(43, 5),
    arc_radius=18,
)


def scale_point(point: Point, factor: float) -> Point:
    return Point(point.x * factor, point.y * factor)


def center_shift(points: tuple[Point, ...], canvas_size: float) -> tuple[float, float]:
    min_x = min(point.x for point in points)
    max_x = max(point.x for point in points)
    min_y = min(point.y for point in points)
    max_y = max(point.y for point in points)
    width = max_x - min_x
    height = max_y - min_y
    return (
        canvas_size / 2 - (min_x + width / 2),
        canvas_size / 2 - (min_y + height / 2),
    )


def transform(point: Point, dx: float, dy: float) -> Point:
    return Point(point.x + dx, point.y + dy)


def fmt(value: float) -> str:
    rendered = f"{value:.3f}".rstrip("0").rstrip(".")
    return rendered or "0"


def symbol_paths(
    *,
    canvas_size: float,
    scale: float,
    offset_x: float = 0,
    offset_y: float = 0,
) -> tuple[str, str]:
    scaled_points = tuple(scale_point(point, scale) for point in BASE_SYMBOL.points)
    dx, dy = center_shift(scaled_points, canvas_size)
    dx += offset_x
    dy += offset_y

    top_start = transform(scale_point(BASE_SYMBOL.top_start, scale), dx, dy)
    top_line_end = transform(scale_point(BASE_SYMBOL.top_line_end, scale), dx, dy)
    top_arc_end = transform(scale_point(BASE_SYMBOL.top_arc_end, scale), dx, dy)
    top_vertical_end = transform(scale_point(BASE_SYMBOL.top_vertical_end, scale), dx, dy)
    bottom_start = transform(scale_point(BASE_SYMBOL.bottom_start, scale), dx, dy)
    bottom_line_end = transform(scale_point(BASE_SYMBOL.bottom_line_end, scale), dx, dy)
    bottom_arc_end = transform(scale_point(BASE_SYMBOL.bottom_arc_end, scale), dx, dy)
    bottom_vertical_end = transform(scale_point(BASE_SYMBOL.bottom_vertical_end, scale), dx, dy)
    check_start = transform(scale_point(BASE_SYMBOL.check_start, scale), dx, dy)
    check_mid = transform(scale_point(BASE_SYMBOL.check_mid, scale), dx, dy)
    check_end = transform(scale_point(BASE_SYMBOL.check_end, scale), dx, dy)
    radius = BASE_SYMBOL.arc_radius * scale

    dark_path = (
        f"M{fmt(top_start.x)} {fmt(top_start.y)} "
        f"H{fmt(top_line_end.x)} "
        f"A{fmt(radius)} {fmt(radius)} 0 0 1 {fmt(top_arc_end.x)} {fmt(top_arc_end.y)} "
        f"V{fmt(top_vertical_end.y)} "
        f"M{fmt(bottom_start.x)} {fmt(bottom_start.y)} "
        f"H{fmt(bottom_line_end.x)} "
        f"A{fmt(radius)} {fmt(radius)} 0 0 1 {fmt(bottom_arc_end.x)} {fmt(bottom_arc_end.y)} "
        f"V{fmt(bottom_vertical_end.y)}"
    )
    accent_path = (
        f"M{fmt(check_start.x)} {fmt(check_start.y)} "
        f"L{fmt(check_mid.x)} {fmt(check_mid.y)} "
        f"L{fmt(check_end.x)} {fmt(check_end.y)}"
    )
    return dark_path, accent_path


def render_full_icon(size: int, symbol_scale: float) -> str:
    dark_path, accent_path = symbol_paths(canvas_size=size, scale=symbol_scale)
    corner = size * 0.28125
    inset = size * 0.03125
    stroke_width = size * 0.01171875
    icon_stroke = size * 0.0625
    check_stroke = size * 0.0625

    return f"""<svg width="{size}" height="{size}" viewBox="0 0 {size} {size}" fill="none" xmlns="http://www.w3.org/2000/svg">
  <rect x="{fmt(inset)}" y="{fmt(inset)}" width="{fmt(size - inset * 2)}" height="{fmt(size - inset * 2)}" rx="{fmt(corner)}" fill="{TILE}"/>
  <rect x="{fmt(inset + stroke_width / 2)}" y="{fmt(inset + stroke_width / 2)}" width="{fmt(size - 2 * inset - stroke_width)}" height="{fmt(size - 2 * inset - stroke_width)}" rx="{fmt(corner - stroke_width / 2)}" stroke="{TILE_STROKE}" stroke-width="{fmt(stroke_width)}"/>
  <path d="{dark_path}" stroke="{DARK}" stroke-width="{fmt(icon_stroke)}" stroke-linecap="round"/>
  <path d="{accent_path}" stroke="{ACCENT}" stroke-width="{fmt(check_stroke)}" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"""


def render_mark(size: int, symbol_scale: float) -> str:
    dark_path, accent_path = symbol_paths(canvas_size=size, scale=symbol_scale)
    line = size * 0.0833333333
    return f"""<svg width="{size}" height="{size}" viewBox="0 0 {size} {size}" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path d="{dark_path}" stroke="{DARK}" stroke-width="{fmt(line)}" stroke-linecap="round"/>
  <path d="{accent_path}" stroke="{ACCENT}" stroke-width="{fmt(line)}" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"""


def write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8", newline="\n")


def main() -> None:
    write(REPO_ROOT / "docs/assets/project-mark.svg", render_full_icon(128, 1.0))
    write(REPO_ROOT / "apps/iclass-gui/public/favicon.svg", render_full_icon(64, 0.5))
    write(REPO_ROOT / "apps/iclass-gui/src/assets/brand/app-mark.svg", render_mark(96, 0.72))
    write(
        REPO_ROOT / "apps/iclass-gui/src-tauri/icons/app-icon-source.svg",
        render_full_icon(1024, 8.0),
    )
    print("[generate-icon-svg] updated SVG brand assets")


if __name__ == "__main__":
    main()
