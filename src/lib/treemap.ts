// Squarified treemap layout (Bruls, Huizing, van Wijk). Pure: maps weighted
// items into non-overlapping rectangles that fill the given area, preferring
// near-square tiles.

export interface TreemapItem {
  key: string;
  value: number;
  color: string;
}

export interface TreemapRect {
  key: string;
  color: string;
  value: number;
  x: number;
  y: number;
  w: number;
  h: number;
}

interface Scaled extends TreemapItem {
  area: number;
}

export function squarify(items: TreemapItem[], width: number, height: number): TreemapRect[] {
  if (width <= 0 || height <= 0) return [];
  const positive = items.filter((i) => i.value > 0);
  const total = positive.reduce((s, i) => s + i.value, 0);
  if (total <= 0) return [];

  const data: Scaled[] = positive
    .map((i) => ({ ...i, area: (i.value / total) * width * height }))
    .sort((a, b) => b.area - a.area);

  const rects: TreemapRect[] = [];
  let x = 0;
  let y = 0;
  let w = width;
  let h = height;

  const worst = (row: Scaled[], side: number): number => {
    if (row.length === 0) return Infinity;
    const sum = row.reduce((s, r) => s + r.area, 0);
    const max = Math.max(...row.map((r) => r.area));
    const min = Math.min(...row.map((r) => r.area));
    const side2 = side * side;
    const sum2 = sum * sum;
    return Math.max((side2 * max) / sum2, sum2 / (side2 * min));
  };

  const flush = (row: Scaled[]) => {
    const side = Math.min(w, h);
    if (side <= 0) return;
    const sum = row.reduce((s, r) => s + r.area, 0);
    const thick = sum / side;
    if (w >= h) {
      let cy = y;
      for (const r of row) {
        const th = r.area / thick;
        rects.push({ key: r.key, color: r.color, value: r.value, x, y: cy, w: thick, h: th });
        cy += th;
      }
      x += thick;
      w -= thick;
    } else {
      let cx = x;
      for (const r of row) {
        const tw = r.area / thick;
        rects.push({ key: r.key, color: r.color, value: r.value, x: cx, y, w: tw, h: thick });
        cx += tw;
      }
      y += thick;
      h -= thick;
    }
  };

  let row: Scaled[] = [];
  let i = 0;
  while (i < data.length) {
    const side = Math.min(w, h);
    if (side <= 0) break;
    const candidate = [...row, data[i]];
    if (row.length === 0 || worst(row, side) >= worst(candidate, side)) {
      row = candidate;
      i++;
    } else {
      flush(row);
      row = [];
    }
  }
  if (row.length) flush(row);
  return rects;
}
