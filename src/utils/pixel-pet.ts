import type { PetState } from '@/types/claude'

const W = 48

/** 角色调色板 */
const P = {
  tr: 'rgba(0,0,0,0)',
  w: '#FFFFFF', bk: '#1A1A2E',
  sk1: '#FFF0E0', sk2: '#FFE0C8',
  pk1: '#FFD1E8', pk2: '#FF9CC2', pk3: '#FF6BA0', pk4: '#D4537E',
  pp1: '#E8D8FF', pp2: '#C8A8FF', pp3: '#9B70DB', pp4: '#6B3FA0',
  bl1: '#D0E8FF', bl2: '#80C0FF', bl3: '#4090E0',
  gn1: '#D0FFD8', gn2: '#70E890', gn3: '#30B050', gn4: '#208040',
  or1: '#FFF0D0', or2: '#FFD080', or3: '#FFA030', or4: '#D07020',
  rd1: '#FFD0D0', rd2: '#FF8080', rd3: '#E04040',
  gy2: '#C0C0C0', gy3: '#808080',
  ye1: '#FFFFD0', ye2: '#FFFF70', ye3: '#FFD700',
  dkbl: '#1A2744',
  teal2: '#60E0C0',
  blush: '#FFB0B0',
  // dark theme 暗系
  dk1: '#0D1117', dk2: '#161B22', dk3: '#21262D', dk4: '#30363D',
  cr1: '#FF4444', cr2: '#CC2222', cr3: '#FF8888',
  // cyber theme 赛博
  cy1: '#00FFFF', cy2: '#00B8D4', cy3: '#006680',
  mt1: '#D0D8E0', mt2: '#8898A8', mt3: '#404850',
  neo: '#00FF88',
} as const

/** 状态特效调色板 — 每种状态 3 档色 (亮→深) */
const S = {
  // thinking  蓝
  t1: '#dbeafe', t2: '#93c5fd', t3: '#3b82f6',
  // coding    绿
  c1: '#bbf7d0', c2: '#4ade80', c3: '#16a34a',
  // success   金
  s1: '#fef9c3', s2: '#fde047', s3: '#eab308',
  // error     红
  e1: '#fecaca', e2: '#f87171', e3: '#dc2626',
  // waiting   橙
  w1: '#ffedd5', w2: '#fb923c', w3: '#ea580c',
  // sleeping  紫
  z1: '#ede9fe', z2: '#a78bfa', z3: '#7c3aed',
} as const

type Ctx = CanvasRenderingContext2D

function px(c: Ctx, x: number, y: number, co: string) {
  if (!co || co === P.tr || x < 0 || x >= W || y < 0 || y >= W) return
  c.fillStyle = co
  c.fillRect(x, y, 1, 1)
}
function row(c: Ctx, x: number, y: number, cols: (string | 0)[]) {
  for (let i = 0; i < cols.length; i++) {
    const co = cols[i]
    if (co) px(c, x + i, y, co)
  }
}
function rect(c: Ctx, x: number, y: number, w: number, h: number, co: string) {
  c.fillStyle = co
  c.fillRect(x, y, w, h)
}

/* ─── 像素画小图形辅助 ─── */

/** 十字星 ✦ (5px) */
function star5(c: Ctx, x: number, y: number, co: string) {
  px(c, x, y - 1, co); px(c, x - 1, y, co); px(c, x, y, co); px(c, x + 1, y, co); px(c, x, y + 1, co)
}
/** 菱形 ◇ (4px) */
function diamond(c: Ctx, x: number, y: number, co: string) {
  px(c, x, y - 1, co); px(c, x - 1, y, co); px(c, x + 1, y, co); px(c, x, y + 1, co)
}
/** 小像素字母 Z (3×3) */
function letterZ(c: Ctx, x: number, y: number, co: string) {
  row(c, x, y, [co, co, co]); px(c, x + 1, y + 1, co); row(c, x, y + 2, [co, co, co])
}

/* ================================================================
   状态特效 — 在角色绘制完成后叠加
   ================================================================ */

function drawStateEffects(c: Ctx, f: number, st: PetState) {
  switch (st) {
    case 'idle':     fxIdle(c, f); break
    case 'thinking': fxThinking(c, f); break
    case 'coding':   fxCoding(c, f); break
    case 'success':  fxSuccess(c, f); break
    case 'error':    fxError(c, f); break
    case 'waiting':  fxWaiting(c, f); break
    case 'sleeping': fxSleeping(c, f); break
  }
}

/** idle: 偶尔闪一颗小光点 */
function fxIdle(c: Ctx, f: number) {
  if (f % 12 < 2) px(c, 6 + (f * 7) % 36, 4 + (f * 3) % 10, '#ffffff60')
}

/** thinking: 蓝色思考气泡 + 旋转光点 */
function fxThinking(c: Ctx, f: number) {
  const { t1, t2, t3 } = S
  // ─ 气泡从小到大上升 (右上方)
  const p = f % 8
  px(c, 37, 14 - (p >> 1), t2)                     // 小点
  const my = 10 - (p >> 1)
  px(c, 40, my, t3); px(c, 41, my, t3)             // 中 2×1
  px(c, 40, my + 1, t2); px(c, 41, my + 1, t2)
  // 大气泡：圆角矩形 + 里面三个跳动省略号
  const by = 1
  rect(c, 39, by, 7, 5, t3)
  px(c, 39, by, P.tr); px(c, 45, by, P.tr)         // 圆角裁掉
  px(c, 39, by + 4, P.tr); px(c, 45, by + 4, P.tr)
  rect(c, 40, by + 1, 5, 3, t1)                    // 内部亮色
  // 三个省略号依次亮
  const dp = f % 4
  if (dp >= 1) px(c, 41, by + 2, t3)
  if (dp >= 2) px(c, 42, by + 2, t3)
  if (dp >= 3) px(c, 43, by + 2, t3)
  // ─ 左侧旋转光点
  const angles = [0, 2.09, 4.19] // 120° 间隔
  angles.forEach((a, i) => {
    const ang = a + f * 0.5
    const ox = 10 + Math.round(6 * Math.cos(ang))
    const oy = 14 + Math.round(6 * Math.sin(ang))
    px(c, ox, oy, [t1, t2, t3][i])
  })
}

/** coding: 绿色终端 + 飞行代码字符 */
function fxCoding(c: Ctx, f: number) {
  const { c1, c2, c3 } = S
  // ─ 终端屏幕 (右侧)
  rect(c, 35, 16, 12, 10, '#0f172a')               // 外框深蓝
  rect(c, 36, 17, 10, 8, '#1e293b')                 // 屏幕背景
  // 滚动代码行
  for (let l = 0; l < 4; l++) {
    const lw = 2 + ((f + l * 3) % 5)                // 行长变化
    const ly = 18 + l * 2
    const col = l === (f % 4) ? c1 : c2             // 当前行高亮
    for (let k = 0; k < lw && k < 8; k++) px(c, 37 + k, ly, col)
  }
  // 光标闪烁
  if (f % 2 === 0) {
    const cursorLine = f % 4
    const cursorX = 37 + 2 + ((f + cursorLine * 3) % 5)
    px(c, cursorX, 18 + cursorLine * 2, c1)
    px(c, cursorX, 19 + cursorLine * 2, c1)
  }
  // ─ 飞行代码字符 (从终端往左飞)
  const chars = [[33, 20], [30, 18], [27, 22], [24, 16]] as const
  chars.forEach(([bx, by], i) => {
    const ox = bx - ((f + i * 2) % 6)
    if (ox > 6) {
      px(c, ox, by, c3)
      px(c, ox + 1, by, c2)
    }
  })
  // ─ 左侧 matrix 风格下落点
  for (let col = 0; col < 3; col++) {
    const mx = 3 + col * 3
    const my = (f * 2 + col * 5) % 20 + 10
    px(c, mx, my, c3)
    px(c, mx, my - 1, c2)
    px(c, mx, my - 2, c1)
  }
}

/** success: 金色星星爆炸 + 五彩纸屑 */
function fxSuccess(c: Ctx, f: number) {
  const { s1, s2, s3 } = S
  // ─ 8 方向星星从中心扩散
  const cx = 24, cy = 20
  const spread = Math.min((f % 10) * 1.5, 12)      // 扩散半径
  const dirs = [[-1, -1], [0, -1], [1, -1], [1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0]]
  dirs.forEach(([dx, dy], i) => {
    const sx = Math.round(cx + dx * spread)
    const sy = Math.round(cy + dy * spread)
    const col = [s1, s2, s3][i % 3]
    if (spread > 3) star5(c, sx, sy, col)
    else diamond(c, sx, sy, col)
  })
  // ─ 五彩纸屑下落
  const confetti = ['#f87171', '#fb923c', '#fbbf24', '#4ade80', '#60a5fa', '#a78bfa', '#f472b6']
  for (let i = 0; i < 8; i++) {
    const cx2 = 4 + ((i * 7 + f * 3) % 40)
    const cy2 = ((f * 2 + i * 6) % 30) + 2
    px(c, cx2, cy2, confetti[i % confetti.length])
    px(c, cx2, cy2 + 1, confetti[(i + 3) % confetti.length])
  }
  // ─ 顶部光芒
  if (f % 3 < 2) {
    star5(c, 10, 3, s2)
    star5(c, 38, 5, s3)
    diamond(c, 6, 12, s1)
    diamond(c, 42, 10, s2)
  }
}

/** error: 红色警告三角 + 闪电 + 震动十字 */
function fxError(c: Ctx, f: number) {
  const { e1, e2, e3 } = S
  // ─ 警告三角 (右上角，闪烁)
  if (f % 3 < 2) {
    //     *
    //    * *
    //   *   *
    //   *****
    const tx = 38, ty = 1
    px(c, tx + 3, ty, e3)
    px(c, tx + 2, ty + 1, e3); px(c, tx + 4, ty + 1, e3)
    px(c, tx + 1, ty + 2, e3); px(c, tx + 5, ty + 2, e3)
    row(c, tx + 1, ty + 3, [e3, e3, e3, e3, e3])
    // 内部填充
    px(c, tx + 2, ty + 2, e1); px(c, tx + 3, ty + 2, e1); px(c, tx + 4, ty + 2, e1)
    px(c, tx + 3, ty + 1, e1)
    // 感叹号
    px(c, tx + 3, ty + 1, e3)
    px(c, tx + 3, ty + 2, e3)
  } else {
    // 闪烁交替：画更小的叹号
    px(c, 41, 2, e2); px(c, 41, 4, e2)
  }
  // ─ 左侧闪电
  const lx = 5, ly = 6
  px(c, lx + 2, ly, e2); px(c, lx + 1, ly + 1, e2); px(c, lx + 2, ly + 2, e3)
  px(c, lx + 3, ly + 2, e3); px(c, lx + 1, ly + 3, e3); px(c, lx, ly + 4, e2)
  // 右侧闪电 (镜像)
  const rx = 42, ry = 12
  px(c, rx, ry, e2); px(c, rx + 1, ry + 1, e2); px(c, rx, ry + 2, e3)
  px(c, rx - 1, ry + 2, e3); px(c, rx + 1, ry + 3, e3); px(c, rx + 2, ry + 4, e2)
  // ─ 红色震动线 (角色两侧)
  const shake = f % 2
  px(c, 10 - shake, 16, e2); px(c, 10 - shake, 18, e2); px(c, 10 - shake, 20, e2)
  px(c, 37 + shake, 16, e2); px(c, 37 + shake, 18, e2); px(c, 37 + shake, 20, e2)
  // ─ 红色 X 标记 (左上)
  if (f % 4 < 3) {
    px(c, 3, 3, e3); px(c, 5, 3, e3)
    px(c, 4, 4, e3)
    px(c, 3, 5, e3); px(c, 5, 5, e3)
  }
}

/** waiting: 橙色跳动加载点 + 沙漏 */
function fxWaiting(c: Ctx, f: number) {
  const { w1, w2, w3 } = S
  // ─ 三个加载点 (头顶上方，依次跳动)
  const dotPhase = f % 6
  for (let i = 0; i < 3; i++) {
    const dx = 20 + i * 4, dy = 2
    const active = dotPhase >= i * 2 && dotPhase < i * 2 + 2
    if (active) {
      // 跳起：画 2×2 + 上移
      px(c, dx, dy - 1, w3); px(c, dx + 1, dy - 1, w3)
      px(c, dx, dy, w2); px(c, dx + 1, dy, w2)
    } else {
      // 静止：画 1×1
      px(c, dx, dy + 1, w2)
    }
  }
  // ─ 沙漏 (右侧)
  const hx = 40, hy = 10
  const flip = f % 8 < 4 // 翻转状态
  // 外框
  row(c, hx, hy, [w3, w3, w3, w3, w3])
  row(c, hx, hy + 6, [w3, w3, w3, w3, w3])
  if (flip) {
    // 上半满 → 下半空
    px(c, hx + 1, hy + 1, w2); px(c, hx + 2, hy + 1, w2); px(c, hx + 3, hy + 1, w2)
    px(c, hx + 2, hy + 2, w2)
    px(c, hx + 2, hy + 3, w1)
    px(c, hx + 2, hy + 4, w1)
    px(c, hx + 1, hy + 5, w1); px(c, hx + 2, hy + 5, w1); px(c, hx + 3, hy + 5, w1)
  } else {
    // 上半空 → 下半满
    px(c, hx + 1, hy + 1, w1); px(c, hx + 2, hy + 1, w1); px(c, hx + 3, hy + 1, w1)
    px(c, hx + 2, hy + 2, w1)
    px(c, hx + 2, hy + 3, w2)
    px(c, hx + 2, hy + 4, w2)
    px(c, hx + 1, hy + 5, w2); px(c, hx + 2, hy + 5, w2); px(c, hx + 3, hy + 5, w2)
  }
  // 中间漏斗颈
  px(c, hx + 2, hy + 3, f % 2 === 0 ? w3 : w1) // 沙粒下落闪烁
  // ─ 左侧小时钟刻度环
  const ticks = [[6, 4], [9, 3], [10, 6], [9, 9], [6, 10], [3, 9], [2, 6], [3, 3]]
  const hand = f % 8
  ticks.forEach((t, i) => {
    px(c, t[0], t[1], i === hand ? w3 : w1)
  })
}

/** sleeping: 紫色 ZZZ + 月亮 + 星星 */
function fxSleeping(c: Ctx, f: number) {
  const { z1, z2, z3 } = S
  // ─ 三个 Z 从小到大上升 (右侧)
  const drift = (f % 6)
  // 小 z (1 像素)
  px(c, 36, 12 - (drift >> 1), z2)
  // 中 Z (3×3)
  letterZ(c, 39, 7 - (drift >> 1), z3)
  // 大 Z (3×3，更亮)
  letterZ(c, 42, 2 - (drift >> 2), z2)
  // 额外：最大的 Z 有光晕
  if (f % 3 < 2) {
    px(c, 41, 1 - (drift >> 2), z1)
    px(c, 45, 3 - (drift >> 2), z1)
  }
  // ─ 月牙 (左上角)
  const mx = 4, my = 2
  px(c, mx + 1, my, z2); px(c, mx + 2, my, z2)
  px(c, mx, my + 1, z2)
  px(c, mx, my + 2, z2)
  px(c, mx + 1, my + 3, z2); px(c, mx + 2, my + 3, z2)
  // 月牙内侧挖空 (用背景色模拟)
  px(c, mx + 1, my + 1, z1); px(c, mx + 2, my + 1, P.tr)
  px(c, mx + 1, my + 2, z1); px(c, mx + 2, my + 2, P.tr)
  // ─ 星星闪烁
  const stars = [[8, 1], [2, 8], [10, 5], [6, 10]] as const
  stars.forEach(([sx, sy], i) => {
    if ((f + i * 2) % 4 < 3) {
      diamond(c, sx, sy, (f + i) % 2 === 0 ? z1 : z2)
    }
  })
}

/* ================================================================
   皮肤定义
   ================================================================ */

export interface SkinDef {
  name: string
  desc: string
  color: string
  draw: (c: Ctx, f: number, st: PetState) => void
}

export const SKINS: Record<string, SkinDef> = {

mahou: {
  name: 'Sakura', desc: 'Magical girl', color: '#FF9CC2',
  draw(c, f, st) {
    const _ = P, y = f % 2 === 0 ? 0 : -1, e = st
    c.clearRect(0, 0, W, W)
    // hair back
    for (let i = 16; i <= 31; i++) for (let j = 8 + y; j <= 28 + y; j++) { if (Math.abs(i - 23.5) < 9 - (j - 8) * 0.3) px(c, i, j, _.pk2) }
    // twin tails
    for (let j = 16 + y; j <= 38 + y; j++) {
      const sw = Math.max(1, 4 - Math.floor((j - 16) / 6))
      for (let dx = 0; dx < sw; dx++) { px(c, 11 - dx, j, _.pk2); px(c, 36 + dx, j, _.pk2) }
    }
    // ribbons
    px(c, 9, 14 + y, _.rd2); px(c, 10, 13 + y, _.rd2); px(c, 10, 15 + y, _.rd2)
    px(c, 37, 14 + y, _.rd2); px(c, 38, 13 + y, _.rd2); px(c, 38, 15 + y, _.rd2)
    // face
    for (let i = 16; i <= 31; i++) for (let j = 10 + y; j <= 23 + y; j++) { if (Math.abs(i - 23.5) < 8.5 - (j < 12 ? 1 : 0)) px(c, i, j, _.sk1) }
    // bangs
    for (let i = 14; i <= 33; i++) { px(c, i, 8 + y, _.pk3); px(c, i, 9 + y, _.pk3) }
    for (let i = 15; i <= 32; i++) px(c, i, 10 + y, _.pk3)
    for (let i = 16; i <= 19; i++) px(c, i, 11 + y, _.pk2)
    for (let i = 28; i <= 31; i++) px(c, i, 11 + y, _.pk2)
    px(c, 22, 11 + y, _.pk2); px(c, 25, 11 + y, _.pk2)
    // hair top
    for (let i = 17; i <= 30; i++) px(c, i, 7 + y, _.pk3)
    for (let i = 19; i <= 28; i++) px(c, i, 6 + y, _.pk3)
    // eyes
    if (e === 'sleeping') {
      row(c, 18, 16 + y, [_.bk, _.bk, _.bk]); row(c, 27, 16 + y, [_.bk, _.bk, _.bk])
    } else if (e === 'error') {
      px(c, 18, 15 + y, _.rd3); px(c, 20, 15 + y, _.rd3); px(c, 19, 16 + y, _.rd3)
      px(c, 27, 15 + y, _.rd3); px(c, 29, 15 + y, _.rd3); px(c, 28, 16 + y, _.rd3)
    } else {
      rect(c, 17, 14 + y, 4, 4, _.w); rect(c, 26, 14 + y, 4, 4, _.w)
      rect(c, 18, 15 + y, 3, 3, _.pp3); rect(c, 27, 15 + y, 3, 3, _.pp3)
      rect(c, 18, 15 + y, 2, 2, _.pp4); rect(c, 27, 15 + y, 2, 2, _.pp4)
      px(c, 18, 14 + y, _.w); px(c, 27, 14 + y, _.w)
      px(c, 19, 14 + y, _.w); px(c, 28, 14 + y, _.w)
      px(c, 19, 15 + y, _.w); px(c, 28, 15 + y, _.w)
      row(c, 17, 13 + y, [_.bk, _.bk, _.bk, _.bk]); row(c, 26, 13 + y, [_.bk, _.bk, _.bk, _.bk])
      if (e === 'success') { px(c, 17, 18 + y, _.bk); px(c, 20, 18 + y, _.bk); px(c, 26, 18 + y, _.bk); px(c, 29, 18 + y, _.bk) }
    }
    // blush
    px(c, 15, 18 + y, _.blush); px(c, 16, 18 + y, _.blush)
    px(c, 31, 18 + y, _.blush); px(c, 32, 18 + y, _.blush)
    // mouth
    if (e === 'success') { px(c, 22, 20 + y, _.bk); px(c, 23, 21 + y, _.bk); px(c, 24, 21 + y, _.bk); px(c, 25, 20 + y, _.bk) }
    else if (e === 'error') { px(c, 22, 21 + y, _.bk); px(c, 23, 20 + y, _.bk); px(c, 24, 20 + y, _.bk); px(c, 25, 21 + y, _.bk) }
    else { px(c, 23, 20 + y, _.bk); px(c, 24, 20 + y, _.bk) }
    // dress body
    for (let i = 18; i <= 29; i++) for (let j = 24 + y; j <= 34 + y; j++) {
      const d = Math.abs(i - 23.5), mw = 6 + Math.floor((j - 24) * 0.5)
      if (d < mw) px(c, i, j, j < 28 + y ? _.pk1 : _.pk2)
    }
    // ribbon on dress
    px(c, 23, 25 + y, _.rd2); px(c, 24, 25 + y, _.rd2); px(c, 22, 26 + y, _.rd2); px(c, 25, 26 + y, _.rd2)
    // collar
    for (let i = 20; i <= 27; i++) px(c, i, 24 + y, _.w)
    // arms
    for (let j = 26 + y; j <= 30 + y; j++) { px(c, 16, j, _.sk1); px(c, 17, j, _.sk1); px(c, 30, j, _.sk1); px(c, 31, j, _.sk1) }
    // legs
    for (let j = 35 + y; j <= 38 + y; j++) { px(c, 20, j, _.sk1); px(c, 21, j, _.sk1); px(c, 26, j, _.sk1); px(c, 27, j, _.sk1) }
    // shoes
    for (let j = 39 + y; j <= 40 + y; j++) { px(c, 19, j, _.pk4); px(c, 20, j, _.pk4); px(c, 21, j, _.pk4); px(c, 26, j, _.pk4); px(c, 27, j, _.pk4); px(c, 28, j, _.pk4) }
    // wand (coding/thinking)
    if (e === 'coding' || e === 'thinking') {
      const wx = 33 + ((f >> 1) % 2), wy = 18 + y
      for (let j = wy; j <= wy + 10; j++) px(c, wx, j, _.ye3)
      px(c, wx - 1, wy - 1, _.ye2); px(c, wx, wy - 1, _.ye2); px(c, wx + 1, wy - 1, _.ye2); px(c, wx, wy - 2, _.ye2)
      if (f % 3 < 2) { px(c, wx - 2, wy - 3, _.pk1); px(c, wx + 2, wy - 3, _.pk1); px(c, wx, wy - 4, _.pk1) }
    }
  },
},

neko: {
  name: 'Mochi', desc: 'Curious kitty', color: '#FFD080',
  draw(c, f, st) {
    const _ = P, y = f % 2 === 0 ? 0 : -1, e = st
    c.clearRect(0, 0, W, W)
    // tail
    const tx = 34 + (f % 2), ty = 30 + y
    for (let j = 0; j < 8; j++) px(c, tx + Math.round(Math.sin(j * 0.8 + (f % 4) * 0.5) * 2), ty + j, _.or3)
    // body
    for (let i = 18; i <= 29; i++) for (let j = 26 + y; j <= 36 + y; j++) { if (Math.abs(i - 23.5) < 6.5) px(c, i, j, _.or2) }
    // belly
    for (let i = 21; i <= 26; i++) for (let j = 28 + y; j <= 34 + y; j++) px(c, i, j, _.ye1)
    // head
    for (let i = 14; i <= 33; i++) for (let j = 10 + y; j <= 24 + y; j++) {
      const cx = 23.5, cy = 17 + y, dx = i - cx, dy = j - cy
      if (dx * dx / 100 + dy * dy / 56 < 1) px(c, i, j, _.or2)
    }
    // ears
    const ear = [[14, 10], [15, 8], [16, 7], [17, 6], [18, 6], [14, 11], [15, 9], [16, 8]]
    ear.forEach(p => { px(c, p[0], p[1] + y, _.or3); px(c, W - 1 - p[0], p[1] + y, _.or3) })
    px(c, 16, 9 + y, _.pk1); px(c, 17, 8 + y, _.pk1); px(c, 31, 9 + y, _.pk1); px(c, 30, 8 + y, _.pk1)
    // face white patch
    for (let i = 18; i <= 29; i++) for (let j = 16 + y; j <= 22 + y; j++) { if (Math.abs(i - 23.5) < 5 && j > 17 + y) px(c, i, j, _.w) }
    // eyes
    if (e === 'sleeping') {
      row(c, 17, 16 + y, [0, _.bk, _.bk, _.bk]); row(c, 27, 16 + y, [_.bk, _.bk, _.bk])
    } else {
      rect(c, 17, 14 + y, 4, 5, _.w); rect(c, 27, 14 + y, 4, 5, _.w)
      rect(c, 18, 15 + y, 3, 3, _.gn3); rect(c, 28, 15 + y, 3, 3, _.gn3)
      rect(c, 19, 15 + y, 2, 2, _.gn4); rect(c, 29, 15 + y, 2, 2, _.gn4)
      px(c, 18, 14 + y, _.w); px(c, 28, 14 + y, _.w)
      px(c, 19, 15 + y, _.w); px(c, 29, 15 + y, _.w)
      row(c, 17, 13 + y, [_.bk, _.bk, _.bk, _.bk]); row(c, 27, 13 + y, [_.bk, _.bk, _.bk, _.bk])
      if (e === 'success') { px(c, 17, 19 + y, _.bk); px(c, 20, 19 + y, _.bk); px(c, 27, 19 + y, _.bk); px(c, 30, 19 + y, _.bk) }
      px(c, 19, 16 + y, _.bk); px(c, 29, 16 + y, _.bk)
    }
    // nose & mouth
    px(c, 23, 19 + y, _.pk3); px(c, 24, 19 + y, _.pk3)
    if (e === 'success') { px(c, 22, 21 + y, _.bk); px(c, 23, 22 + y, _.bk); px(c, 24, 22 + y, _.bk); px(c, 25, 21 + y, _.bk); px(c, 23, 21 + y, _.bk); px(c, 24, 21 + y, _.bk) }
    else { px(c, 23, 20 + y, _.bk); px(c, 22, 21 + y, _.bk); px(c, 24, 20 + y, _.bk); px(c, 25, 21 + y, _.bk) }
    // whiskers
    row(c, 8, 18 + y, [_.gy3, 0, _.gy3, 0, _.gy3]); row(c, 35, 18 + y, [_.gy3, 0, _.gy3, 0, _.gy3])
    row(c, 9, 20 + y, [_.gy3, 0, _.gy3, 0, _.gy3]); row(c, 34, 20 + y, [_.gy3, 0, _.gy3, 0, _.gy3])
    // paws
    for (let j = 34 + y; j <= 38 + y; j++) { rect(c, 18, j, 3, 1, _.or2); rect(c, 27, j, 3, 1, _.or2) }
    px(c, 18, 38 + y, _.w); px(c, 19, 38 + y, _.w); px(c, 27, 38 + y, _.w); px(c, 28, 38 + y, _.w)
    for (let j = 36 + y; j <= 38 + y; j++) { px(c, 15, j, _.or3); px(c, 32, j, _.or3) }
    // coding: laptop
    if (e === 'coding') { rect(c, 35, 20 + y, 10, 8, _.dkbl); for (let j = 0; j < 3; j++) { const lw = 2 + (f + j) % 4; for (let k = 0; k < lw; k++) px(c, 36 + k, 22 + j * 2 + y, _.gn2) } }
    px(c, 14, 19 + y, _.blush); px(c, 33, 19 + y, _.blush)
  },
},

kitsune: {
  name: 'Kitsune', desc: 'Fox spirit', color: '#FF8040',
  draw(c, f, st) {
    const _ = P, y = f % 2 === 0 ? 0 : -1, e = st; c.clearRect(0, 0, W, W)
    // tail
    for (let j = 18 + y; j <= 40 + y; j++) { const tw = Math.max(1, 6 - Math.abs(j - 28 - y) * 0.4); for (let k = 0; k < tw; k++) { px(c, 36 + k, j, _.or3); if (k < tw - 1) px(c, 37 + k, j, _.or2) } }
    for (let j = 22 + y; j <= 36 + y; j++) { const tw2 = Math.max(0, 3 - Math.abs(j - 28 - y) * 0.3); for (let k = 0; k < tw2; k++) px(c, 37 + k, j, _.w) }
    // body
    for (let i = 18; i <= 29; i++) for (let j = 25 + y; j <= 35 + y; j++) px(c, i, j, _.or2)
    for (let i = 19; i <= 28; i++) { px(c, i, 25 + y, _.rd2); px(c, i, 26 + y, _.rd2) }
    px(c, 23, 27 + y, _.ye3); px(c, 24, 27 + y, _.ye3)
    // head
    for (let i = 15; i <= 32; i++) for (let j = 10 + y; j <= 23 + y; j++) { const dx = i - 23.5, dy = j - 16.5 - y; if (dx * dx / 80 + dy * dy / 50 < 1) px(c, i, j, _.or2) }
    // ears
    ;[[13, 10], [14, 8], [15, 6], [16, 5], [17, 4], [18, 5], [13, 11], [14, 9], [15, 7], [16, 6]].forEach(p => { px(c, p[0], p[1] + y, _.or3); px(c, W - 1 - p[0], p[1] + y, _.or3) })
    px(c, 15, 8 + y, _.pk1); px(c, 16, 7 + y, _.pk1); px(c, 32, 8 + y, _.pk1); px(c, 31, 7 + y, _.pk1)
    // face white patch
    for (let i = 19; i <= 28; i++) for (let j = 17 + y; j <= 22 + y; j++) px(c, i, j, _.w)
    // eyes
    if (e === 'sleeping') { row(c, 18, 16 + y, [_.bk, _.bk, _.bk]); row(c, 27, 16 + y, [_.bk, _.bk, _.bk]) }
    else {
      rect(c, 17, 14 + y, 4, 4, _.w); rect(c, 27, 14 + y, 4, 4, _.w)
      rect(c, 18, 15 + y, 3, 3, _.or4); rect(c, 28, 15 + y, 3, 3, _.or4)
      rect(c, 19, 15 + y, 2, 2, _.rd3); rect(c, 29, 15 + y, 2, 2, _.rd3)
      px(c, 18, 14 + y, _.w); px(c, 28, 14 + y, _.w); px(c, 19, 15 + y, _.w); px(c, 29, 15 + y, _.w)
      row(c, 17, 13 + y, [_.bk, _.bk, _.bk, _.bk]); row(c, 27, 13 + y, [_.bk, _.bk, _.bk, _.bk])
    }
    // nose & mouth
    px(c, 23, 19 + y, _.bk); px(c, 24, 19 + y, _.bk)
    if (e === 'success') { px(c, 22, 21 + y, _.bk); px(c, 23, 22 + y, _.bk); px(c, 24, 22 + y, _.bk); px(c, 25, 21 + y, _.bk) }
    else { px(c, 22, 20 + y, _.bk); px(c, 25, 20 + y, _.bk) }
    // face markings
    row(c, 14, 18 + y, [_.rd2, 0, _.rd2]); row(c, 31, 18 + y, [_.rd2, 0, _.rd2])
    // paws
    for (let j = 36 + y; j <= 39 + y; j++) { rect(c, 19, j, 3, 1, _.or3); rect(c, 26, j, 3, 1, _.or3) }
    // fox fire (coding/thinking)
    if (e === 'coding' || e === 'thinking') {
      const colors = [_.bl3, _.bl2, _.teal2]
      for (let i = 0; i < 3; i++) {
        const fx = 8 + i * 4, fy = 12 + y - ((f + i) % 3)
        px(c, fx, fy, colors[i]); px(c, fx, fy - 1, colors[(i + 1) % 3])
      }
    }
  },
},

usagi: {
  name: 'Usagi', desc: 'Fluffy bunny', color: '#FFB0C8',
  draw(c, f, st) {
    const _ = P, y = f % 2 === 0 ? 0 : -1, e = st; c.clearRect(0, 0, W, W)
    // ears
    for (let j = 0 + y; j <= 12 + y; j++) { rect(c, 17, j, 3, 1, _.w); rect(c, 28, j, 3, 1, _.w) }
    px(c, 18, 2 + y, _.pk1); px(c, 18, 4 + y, _.pk1); px(c, 18, 6 + y, _.pk1); px(c, 18, 8 + y, _.pk1)
    px(c, 29, 2 + y, _.pk1); px(c, 29, 4 + y, _.pk1); px(c, 29, 6 + y, _.pk1); px(c, 29, 8 + y, _.pk1)
    if (f % 4 > 1) { px(c, 16, 1 + y, _.w); px(c, 30, 1 + y, _.w) }
    // head
    for (let i = 14; i <= 33; i++) for (let j = 12 + y; j <= 25 + y; j++) { const dx = i - 23.5, dy = j - 18.5 - y; if (dx * dx / 100 + dy * dy / 50 < 1) px(c, i, j, _.w) }
    for (let j = 18 + y; j <= 22 + y; j++) { px(c, 13, j, _.w); px(c, 34, j, _.w) }
    // eyes
    if (e === 'sleeping') { row(c, 18, 18 + y, [_.bk, _.bk, _.bk]); row(c, 27, 18 + y, [_.bk, _.bk, _.bk]) }
    else {
      rect(c, 17, 16 + y, 5, 5, _.w); rect(c, 26, 16 + y, 5, 5, _.w)
      rect(c, 18, 17 + y, 3, 3, _.pk4); rect(c, 27, 17 + y, 3, 3, _.pk4)
      rect(c, 19, 17 + y, 2, 2, _.rd3); rect(c, 28, 17 + y, 2, 2, _.rd3)
      px(c, 19, 16 + y, _.w); px(c, 28, 16 + y, _.w); px(c, 19, 17 + y, _.w); px(c, 28, 17 + y, _.w)
      row(c, 17, 15 + y, [_.bk, _.bk, _.bk, _.bk, _.bk]); row(c, 26, 15 + y, [_.bk, _.bk, _.bk, _.bk, _.bk])
    }
    // nose & mouth
    px(c, 23, 21 + y, _.pk3); px(c, 24, 21 + y, _.pk3)
    if (e === 'success') { px(c, 22, 23 + y, _.bk); px(c, 23, 24 + y, _.bk); px(c, 24, 24 + y, _.bk); px(c, 25, 23 + y, _.bk) }
    else { px(c, 23, 22 + y, _.bk); px(c, 24, 22 + y, _.bk) }
    rect(c, 14, 20 + y, 2, 1, _.blush); rect(c, 32, 20 + y, 2, 1, _.blush)
    // body
    for (let i = 19; i <= 28; i++) for (let j = 26 + y; j <= 36 + y; j++) px(c, i, j, _.w)
    for (let i = 20; i <= 27; i++) px(c, i, 26 + y, _.pk2); px(c, 23, 27 + y, _.pk3); px(c, 24, 27 + y, _.pk3)
    // legs & paws
    for (let j = 34 + y; j <= 38 + y; j++) { rect(c, 17, j, 3, 1, _.w); rect(c, 28, j, 3, 1, _.w) }
    px(c, 17, 38 + y, _.pk1); px(c, 18, 38 + y, _.pk1); px(c, 28, 38 + y, _.pk1); px(c, 29, 38 + y, _.pk1)
    // tail
    rect(c, 24, 35 + y, 4, 3, _.w); px(c, 25, 34 + y, _.w); px(c, 26, 34 + y, _.w)
    // coding: laptop
    if (e === 'coding') { rect(c, 34, 22 + y, 10, 8, _.dkbl); for (let j = 0; j < 3; j++) { const lw = 2 + (f + j) % 4; for (let k = 0; k < lw; k++) px(c, 35 + k, 24 + j * 2 + y, _.gn2) } }
  },
},

devil: {
  name: 'Imp', desc: 'Mischievous devil', color: '#9B70DB',
  draw(c, f, st) {
    const _ = P, y = f % 2 === 0 ? 0 : -1, e = st; c.clearRect(0, 0, W, W)
    // wings
    const wf = f % 4 < 2 ? 0 : 1
    ;[[8, 16], [7, 14], [6, 12], [7, 10], [9, 9], [11, 10], [12, 12]].forEach(p => { px(c, p[0] - wf, p[1] + y, _.pp3); px(c, W - 1 - p[0] + wf, p[1] + y, _.pp3) })
    ;[[9, 14], [8, 12], [9, 11], [10, 11]].forEach(p => { px(c, p[0] - wf, p[1] + y, _.pp2); px(c, W - 1 - p[0] + wf, p[1] + y, _.pp2) })
    // tail
    for (let j = 32 + y; j <= 42 + y; j++) { const ttx = 30 + Math.round(Math.sin((j - 32) * 0.5 + f * 0.3) * 3); px(c, ttx, j, _.pp3) }
    px(c, 30 + Math.round(Math.sin(10 * 0.5 + f * 0.3) * 3), 42 + y, _.rd3)
    // body
    for (let i = 19; i <= 28; i++) for (let j = 24 + y; j <= 34 + y; j++) px(c, i, j, _.pp2)
    // head
    for (let i = 15; i <= 32; i++) for (let j = 10 + y; j <= 23 + y; j++) { const dx = i - 23.5, dy = j - 16.5 - y; if (dx * dx / 80 + dy * dy / 50 < 1) px(c, i, j, _.pp1) }
    // horns
    px(c, 16, 9 + y, _.rd3); px(c, 15, 8 + y, _.rd3); px(c, 14, 7 + y, _.rd2)
    px(c, 31, 9 + y, _.rd3); px(c, 32, 8 + y, _.rd3); px(c, 33, 7 + y, _.rd2)
    // eyes
    if (e === 'sleeping') { row(c, 18, 16 + y, [_.bk, _.bk, _.bk]); row(c, 27, 16 + y, [_.bk, _.bk, _.bk]) }
    else {
      rect(c, 17, 14 + y, 4, 4, _.w); rect(c, 27, 14 + y, 4, 4, _.w)
      rect(c, 18, 15 + y, 3, 3, _.rd3); rect(c, 28, 15 + y, 3, 3, _.rd3)
      rect(c, 19, 16 + y, 1, 1, _.bk); rect(c, 29, 16 + y, 1, 1, _.bk)
      px(c, 18, 14 + y, _.w); px(c, 28, 14 + y, _.w)
      row(c, 17, 13 + y, [_.bk, _.bk, _.bk, _.bk]); row(c, 27, 13 + y, [_.bk, _.bk, _.bk, _.bk])
    }
    // mouth
    if (e === 'success') { px(c, 21, 20 + y, _.bk); px(c, 22, 21 + y, _.bk); px(c, 23, 21 + y, _.bk); px(c, 24, 21 + y, _.bk); px(c, 25, 21 + y, _.bk); px(c, 26, 20 + y, _.bk) }
    else { px(c, 22, 20 + y, _.bk); px(c, 23, 20 + y, _.bk); px(c, 25, 20 + y, _.bk); px(c, 26, 19 + y, _.bk) }
    // legs & shoes
    for (let j = 35 + y; j <= 39 + y; j++) { rect(c, 20, j, 3, 1, _.pp2); rect(c, 25, j, 3, 1, _.pp2) }
    px(c, 20, 39 + y, _.pp4); px(c, 21, 39 + y, _.pp4); px(c, 25, 39 + y, _.pp4); px(c, 26, 39 + y, _.pp4)
    // coding: laptop (red tint)
    if (e === 'coding') { rect(c, 34, 18 + y, 10, 8, _.dkbl); for (let j = 0; j < 3; j++) { const lw = 2 + (f + j) % 4; for (let k = 0; k < lw; k++) px(c, 35 + k, 20 + j * 2 + y, _.rd1) } }
  },
},

elf: {
  name: 'Sylph', desc: 'Forest spirit', color: '#70E890',
  draw(c, f, st) {
    const _ = P, y = f % 2 === 0 ? 0 : -1, e = st; c.clearRect(0, 0, W, W)
    // leaf wings
    const lf = f % 4 < 2 ? 0 : 1
    ;[[10, 14], [9, 12], [10, 10], [12, 9], [10, 16], [9, 18]].forEach(p => { px(c, p[0] - lf, p[1] + y, _.gn2); px(c, W - 1 - p[0] + lf, p[1] + y, _.gn2) })
    for (let j = 8 + y; j <= 32 + y; j++) { const hw = Math.max(1, 3 - Math.floor(Math.abs(j - 18 - y) / 5)); px(c, 13 - hw, j, _.gn3); px(c, 14 - hw, j, _.gn2); px(c, 33 + hw, j, _.gn3); px(c, 34 + hw, j, _.gn2) }
    // body
    for (let i = 19; i <= 28; i++) for (let j = 24 + y; j <= 35 + y; j++) px(c, i, j, j < 28 + y ? _.gn1 : _.gn2)
    // head
    for (let i = 15; i <= 32; i++) for (let j = 10 + y; j <= 23 + y; j++) { const dx = i - 23.5, dy = j - 16.5 - y; if (dx * dx / 80 + dy * dy / 50 < 1) px(c, i, j, _.sk1) }
    // pointed ears
    px(c, 12, 14 + y, _.sk2); px(c, 11, 15 + y, _.sk2); px(c, 10, 16 + y, _.sk2); px(c, 9, 17 + y, _.sk1)
    px(c, 35, 14 + y, _.sk2); px(c, 36, 15 + y, _.sk2); px(c, 37, 16 + y, _.sk2); px(c, 38, 17 + y, _.sk1)
    // hair/leaves
    for (let i = 16; i <= 31; i++) { px(c, i, 8 + y, _.gn3); px(c, i, 9 + y, _.gn3) }
    for (let i = 15; i <= 32; i++) px(c, i, 10 + y, _.gn2)
    for (let i = 16; i <= 20; i++) px(c, i, 11 + y, _.gn2)
    for (let i = 27; i <= 31; i++) px(c, i, 11 + y, _.gn2)
    for (let i = 18; i <= 29; i++) px(c, i, 7 + y, _.gn3)
    px(c, 18, 7 + y, _.pk2); px(c, 21, 6 + y, _.ye3); px(c, 24, 6 + y, _.pk2); px(c, 27, 7 + y, _.ye3); px(c, 29, 7 + y, _.pk2)
    // eyes
    if (e === 'sleeping') { row(c, 18, 16 + y, [_.bk, _.bk, _.bk]); row(c, 27, 16 + y, [_.bk, _.bk, _.bk]) }
    else {
      rect(c, 17, 14 + y, 4, 4, _.w); rect(c, 27, 14 + y, 4, 4, _.w)
      rect(c, 18, 15 + y, 3, 3, _.gn3); rect(c, 28, 15 + y, 3, 3, _.gn3)
      rect(c, 19, 15 + y, 2, 2, _.gn4); rect(c, 29, 15 + y, 2, 2, _.gn4)
      px(c, 18, 14 + y, _.w); px(c, 28, 14 + y, _.w); px(c, 19, 15 + y, _.w); px(c, 29, 15 + y, _.w)
      row(c, 17, 13 + y, [_.bk, _.bk, _.bk, _.bk]); row(c, 27, 13 + y, [_.bk, _.bk, _.bk, _.bk])
    }
    // blush & mouth
    px(c, 15, 19 + y, _.blush); px(c, 32, 19 + y, _.blush)
    if (e === 'success') { px(c, 22, 20 + y, _.bk); px(c, 23, 21 + y, _.bk); px(c, 24, 21 + y, _.bk); px(c, 25, 20 + y, _.bk) }
    else { px(c, 23, 20 + y, _.bk); px(c, 24, 20 + y, _.bk) }
    // legs & shoes
    for (let j = 36 + y; j <= 39 + y; j++) { rect(c, 20, j, 3, 1, _.sk1); rect(c, 25, j, 3, 1, _.sk1) }
    px(c, 20, 39 + y, _.gn3); px(c, 21, 39 + y, _.gn3); px(c, 25, 39 + y, _.gn3); px(c, 26, 39 + y, _.gn3)
    // coding/thinking: floating spirit lights
    if (e === 'coding' || e === 'thinking') {
      for (let i = 0; i < 3; i++) {
        const fx = 8 + i * 4, fy = 12 + y - ((f + i) % 3)
        px(c, fx, fy, _.gn2); px(c, fx, fy - 1, _.teal2)
      }
    }
  },
},

yami: {
  name: 'Yami', desc: 'Dark reaper', color: '#FF4444',
  draw(c, f, st) {
    const _ = P, y = f % 2 === 0 ? 0 : -1, e = st; c.clearRect(0, 0, W, W)
    const sway = f % 4 < 2 ? 0 : 1
    // cape back (flowing, wider at bottom)
    for (let j = 20 + y; j <= 42 + y; j++) {
      const hw = 5 + Math.floor((j - 20) * 0.45)
      for (let dx = -hw; dx <= hw; dx++) {
        const cx = 24 + dx + (dx > 0 ? sway : -sway)
        if (cx >= 0 && cx < W) px(c, cx, j, Math.abs(dx) > hw - 2 ? _.dk3 : _.dk1)
      }
    }
    // cape tattered edges
    if (sway === 0) { px(c, 15, 41 + y, _.dk3); px(c, 33, 40 + y, _.dk3) }
    else { px(c, 14, 40 + y, _.dk3); px(c, 34, 41 + y, _.dk3) }
    // hood (large, enveloping)
    for (let i = 14; i <= 33; i++) for (let j = 6 + y; j <= 24 + y; j++) {
      const dx = i - 23.5, dy = j - 14 - y
      if (dx * dx / 110 + dy * dy / 80 < 1) px(c, i, j, _.dk2)
    }
    // hood rim (darker outline)
    for (let i = 15; i <= 32; i++) px(c, i, 6 + y, _.dk4)
    for (let i = 16; i <= 31; i++) px(c, i, 5 + y, _.dk3)
    px(c, 14, 8 + y, _.dk4); px(c, 33, 8 + y, _.dk4)
    px(c, 13, 10 + y, _.dk4); px(c, 34, 10 + y, _.dk4)
    // hood peak
    px(c, 23, 4 + y, _.dk3); px(c, 24, 4 + y, _.dk3)
    // face shadow (visible through hood opening)
    for (let i = 19; i <= 28; i++) for (let j = 13 + y; j <= 22 + y; j++) {
      if (Math.abs(i - 23.5) < 5.5 - (j < 14 + y ? 1 : 0)) px(c, i, j, _.dk4)
    }
    // pale skin hint
    for (let i = 20; i <= 27; i++) for (let j = 15 + y; j <= 20 + y; j++) {
      if (Math.abs(i - 23.5) < 4) px(c, i, j, _.mt2)
    }
    // eyes — glowing red
    if (e === 'sleeping') {
      row(c, 19, 17 + y, [_.cr2, _.cr2, _.cr2]); row(c, 26, 17 + y, [_.cr2, _.cr2, _.cr2])
    } else {
      rect(c, 19, 16 + y, 3, 2, _.cr1); rect(c, 26, 16 + y, 3, 2, _.cr1)
      px(c, 20, 16 + y, _.cr3); px(c, 27, 16 + y, _.cr3) // bright pupil center
      // glow halo
      px(c, 18, 16 + y, _.cr2); px(c, 22, 16 + y, _.cr2)
      px(c, 25, 16 + y, _.cr2); px(c, 29, 16 + y, _.cr2)
      if (e === 'error') {
        // eyes flare wider
        px(c, 18, 15 + y, _.cr1); px(c, 22, 15 + y, _.cr1)
        px(c, 25, 15 + y, _.cr1); px(c, 29, 15 + y, _.cr1)
      }
      if (e === 'success') {
        // slight upward curve
        px(c, 19, 18 + y, _.cr2); px(c, 21, 18 + y, _.cr2)
        px(c, 26, 18 + y, _.cr2); px(c, 28, 18 + y, _.cr2)
      }
    }
    // mouth (barely visible)
    if (e === 'success') { px(c, 23, 20 + y, _.mt2); px(c, 24, 20 + y, _.mt2) }
    if (e === 'error') { px(c, 22, 20 + y, _.cr2); px(c, 23, 19 + y, _.cr2); px(c, 24, 19 + y, _.cr2); px(c, 25, 20 + y, _.cr2) }
    // body center line
    for (let j = 24 + y; j <= 38 + y; j++) { px(c, 23, j, _.dk3); px(c, 24, j, _.dk3) }
    // hood side draping over body
    for (let j = 14 + y; j <= 24 + y; j++) {
      px(c, 17, j, _.dk1); px(c, 18, j, _.dk1)
      px(c, 29, j, _.dk1); px(c, 30, j, _.dk1)
    }
    // hands (pale, peeking from sleeves)
    px(c, 16, 30 + y, _.mt2); px(c, 17, 30 + y, _.mt2)
    px(c, 30, 30 + y, _.mt2); px(c, 31, 30 + y, _.mt2)
    // scythe (coding/thinking)
    if (e === 'coding' || e === 'thinking') {
      // handle
      for (let j = 6 + y; j <= 28 + y; j++) px(c, 35, j, _.mt2)
      // blade curve
      px(c, 34, 5 + y, _.gy2); px(c, 33, 4 + y, _.gy2); px(c, 32, 3 + y, _.w)
      px(c, 31, 3 + y, _.w); px(c, 30, 4 + y, _.w); px(c, 29, 5 + y, _.gy2)
      // blade edge glow
      px(c, 32, 2 + y, _.cr1); px(c, 31, 2 + y, _.cr1); px(c, 30, 3 + y, _.cr1)
      if (f % 3 < 2) { px(c, 29, 4 + y, _.cr2); px(c, 33, 3 + y, _.cr2) }
    }
    // ambient dark particles
    if (e !== 'sleeping') {
      const pts = [[10, 8], [38, 12], [8, 28], [40, 24], [12, 38]]
      pts.forEach((p, i) => {
        if ((f + i * 2) % 5 < 2) px(c, p[0] + sway, p[1] + y, _.dk4)
      })
    }
  },
},

nexus: {
  name: 'Nexus', desc: 'Cyber unit', color: '#00FFFF',
  draw(c, f, st) {
    const _ = P, y = f % 2 === 0 ? 0 : -1, e = st; c.clearRect(0, 0, W, W)
    // antenna poles
    px(c, 20, 5 + y, _.mt2); px(c, 20, 6 + y, _.mt2); px(c, 20, 7 + y, _.mt2)
    px(c, 27, 5 + y, _.mt2); px(c, 27, 6 + y, _.mt2); px(c, 27, 7 + y, _.mt2)
    // antenna tips (alternating blink)
    px(c, 20, 4 + y, f % 3 < 2 ? _.cy1 : _.cy3)
    px(c, 27, 4 + y, f % 3 < 2 ? _.cy3 : _.cy1)
    // head (rounded rect: 16px wide, 12px tall)
    rect(c, 16, 9 + y, 16, 12, _.mt3) // outer shell
    rect(c, 17, 8 + y, 14, 14, _.mt3)
    rect(c, 17, 9 + y, 14, 12, _.mt2) // inner
    // head accent lines (side vents)
    px(c, 16, 11 + y, _.cy2); px(c, 16, 13 + y, _.cy2); px(c, 16, 15 + y, _.cy2)
    px(c, 31, 11 + y, _.cy2); px(c, 31, 13 + y, _.cy2); px(c, 31, 15 + y, _.cy2)
    // visor (eye screen)
    rect(c, 18, 13 + y, 12, 5, '#001418') // visor bg
    rect(c, 18, 12 + y, 12, 1, _.cy3) // visor top edge
    rect(c, 18, 18 + y, 12, 1, _.cy3) // visor bottom edge
    if (e === 'sleeping') {
      // ── ── sleeping bars
      row(c, 19, 15 + y, [_.cy3, _.cy3, _.cy3]); row(c, 26, 15 + y, [_.cy3, _.cy3, _.cy3])
    } else if (e === 'error') {
      // X X on visor
      px(c, 19, 14 + y, _.cr1); px(c, 21, 14 + y, _.cr1); px(c, 20, 15 + y, _.cr1)
      px(c, 19, 16 + y, _.cr1); px(c, 21, 16 + y, _.cr1)
      px(c, 26, 14 + y, _.cr1); px(c, 28, 14 + y, _.cr1); px(c, 27, 15 + y, _.cr1)
      px(c, 26, 16 + y, _.cr1); px(c, 28, 16 + y, _.cr1)
    } else {
      // normal eyes (bright rectangles)
      rect(c, 19, 14 + y, 3, 3, _.cy1); rect(c, 26, 14 + y, 3, 3, _.cy1)
      px(c, 20, 14 + y, _.w); px(c, 27, 14 + y, _.w) // highlight
      if (e === 'success') {
        // ^_^ happy — cut bottom corners
        px(c, 19, 16 + y, '#001418'); px(c, 21, 16 + y, '#001418')
        px(c, 26, 16 + y, '#001418'); px(c, 28, 16 + y, '#001418')
      }
    }
    // neck connector
    rect(c, 22, 21 + y, 4, 2, _.mt3)
    px(c, 23, 21 + y, _.cy3); px(c, 24, 21 + y, _.cy3)
    // torso
    rect(c, 17, 23 + y, 14, 12, _.mt3) // outer
    rect(c, 18, 23 + y, 12, 12, _.mt2) // inner
    // chest core (pulsing)
    px(c, 23, 25 + y, f % 2 === 0 ? _.cy1 : _.cy2)
    px(c, 24, 25 + y, f % 2 === 0 ? _.cy1 : _.cy2)
    // chest LED strip (scanning left→right)
    const ledPos = f % 8
    for (let i = 0; i < 8; i++) {
      const dist = Math.abs(i - ledPos)
      const col = dist === 0 ? _.cy1 : dist === 1 ? _.cy2 : _.cy3
      px(c, 20 + i, 28 + y, col)
    }
    // body circuit lines
    px(c, 19, 30 + y, _.cy3); px(c, 20, 30 + y, _.cy3); px(c, 20, 31 + y, _.cy3)
    px(c, 28, 30 + y, _.cy3); px(c, 27, 30 + y, _.cy3); px(c, 27, 31 + y, _.cy3)
    // arms
    rect(c, 13, 24 + y, 4, 8, _.mt3)
    rect(c, 14, 24 + y, 2, 8, _.mt2)
    rect(c, 31, 24 + y, 4, 8, _.mt3)
    rect(c, 31, 24 + y, 2, 8, _.mt2)
    // arm joints (cyan dots)
    px(c, 15, 24 + y, _.cy2); px(c, 15, 28 + y, _.cy2)
    px(c, 32, 24 + y, _.cy2); px(c, 32, 28 + y, _.cy2)
    // hands
    rect(c, 13, 32 + y, 4, 2, _.mt1)
    rect(c, 31, 32 + y, 4, 2, _.mt1)
    // legs
    rect(c, 19, 35 + y, 4, 4, _.mt3)
    rect(c, 20, 35 + y, 2, 4, _.mt2)
    rect(c, 25, 35 + y, 4, 4, _.mt3)
    rect(c, 26, 35 + y, 2, 4, _.mt2)
    // knee joints
    px(c, 20, 35 + y, _.cy2); px(c, 27, 35 + y, _.cy2)
    // feet (wider than legs)
    rect(c, 18, 39 + y, 6, 2, _.mt3)
    rect(c, 24, 39 + y, 6, 2, _.mt3)
    px(c, 18, 40 + y, _.cy2); px(c, 23, 40 + y, _.cy2)
    px(c, 24, 40 + y, _.cy2); px(c, 29, 40 + y, _.cy2)
    // coding: holographic display
    if (e === 'coding') {
      rect(c, 36, 14 + y, 10, 8, _.cy3)
      rect(c, 37, 15 + y, 8, 6, '#001418')
      for (let j = 0; j < 3; j++) {
        const lw = 2 + (f + j * 2) % 5
        for (let k = 0; k < lw && k < 6; k++) px(c, 38 + k, 16 + j * 2 + y, _.neo)
      }
      // projection beam
      px(c, 34, 26 + y, _.cy3); px(c, 35, 22 + y, _.cy3); px(c, 35, 18 + y, _.cy2)
    }
    // thinking: data stream
    if (e === 'thinking') {
      for (let i = 0; i < 3; i++) {
        const tx = 36 + i * 3, ty = 8 + y - ((f + i) % 4)
        px(c, tx, ty, _.cy2); px(c, tx, ty + 1, _.cy1); px(c, tx, ty + 2, _.cy3)
      }
    }
  },
},

}

/* ================================================================
   渲染器
   ================================================================ */

/** 像素宠物渲染器 */
export class PixelPet {
  private canvas: HTMLCanvasElement
  private displayCanvas: HTMLCanvasElement
  private ctx: Ctx
  private displayCtx: Ctx
  private frame = 0
  private animId = 0
  private _skin: string = 'mahou'
  private _state: PetState = 'idle'
  private scale: number
  private _mirror = false
  private _opacity = 1
  /** 帧缓存：key = `${skin}-${state}-${frameIndex}` */
  private frameCache = new Map<string, ImageBitmap>()

  constructor(container: HTMLElement, scale = 4) {
    this.scale = scale
    const sz = Math.round(W * scale)

    // 离屏 canvas (48x48 像素画)
    this.canvas = document.createElement('canvas')
    this.canvas.width = W
    this.canvas.height = W
    const ctx = this.canvas.getContext('2d')
    if (!ctx) throw new Error('Failed to get 2d context for offscreen canvas')
    this.ctx = ctx

    // 显示 canvas (放大后)
    this.displayCanvas = document.createElement('canvas')
    this.displayCanvas.width = sz
    this.displayCanvas.height = sz
    this.displayCanvas.style.width = `${sz}px`
    this.displayCanvas.style.height = `${sz}px`
    this.displayCanvas.style.imageRendering = 'pixelated'
    container.appendChild(this.displayCanvas)

    const displayCtx = this.displayCanvas.getContext('2d')
    if (!displayCtx) throw new Error('Failed to get 2d context for display canvas')
    this.displayCtx = displayCtx
    this.displayCtx.imageSmoothingEnabled = false

    this.startLoop()
  }

  set skin(name: string) { this._skin = name }
  get skin() { return this._skin }

  set state(s: PetState) { this._state = s }
  get state() { return this._state }

  set mirror(v: boolean) { this._mirror = v }
  get mirror() { return this._mirror }

  set opacity(v: number) {
    this._opacity = v
    this.displayCanvas.style.opacity = String(v)
  }
  get opacity() { return this._opacity }

  setScale(newScale: number) {
    this.scale = newScale
    const sz = Math.round(W * newScale)
    this.displayCanvas.width = sz
    this.displayCanvas.height = sz
    this.displayCanvas.style.width = `${sz}px`
    this.displayCanvas.style.height = `${sz}px`
    this.displayCtx = this.displayCanvas.getContext('2d')!
    this.displayCtx.imageSmoothingEnabled = false
  }

  getScale() { return this.scale }

  /** 清除帧缓存（换皮肤时可调用，但通常不需要因为 key 包含 skin） */
  clearCache() {
    this.frameCache.forEach((bmp) => bmp.close())
    this.frameCache.clear()
  }

  private startLoop() {
    const tick = () => {
      this.frame++
      const sf = Math.floor(this.frame / 12)
      const skin = SKINS[this._skin]
      if (!skin) {
        this.animId = requestAnimationFrame(tick)
        return
      }

      const cacheKey = `${this._skin}-${this._state}-${sf}`
      const cached = this.frameCache.get(cacheKey)
      const dsz = Math.round(W * this.scale)

      if (cached) {
        // 缓存命中：直接绘制 ImageBitmap
        this.displayCtx.clearRect(0, 0, dsz, dsz)
        if (this._mirror) {
          this.displayCtx.save()
          this.displayCtx.scale(-1, 1)
          this.displayCtx.drawImage(cached, -dsz, 0, dsz, dsz)
          this.displayCtx.restore()
        } else {
          this.displayCtx.drawImage(cached, 0, 0, dsz, dsz)
        }
      } else {
        // 缓存未命中：绘制到离屏 canvas，然后缓存
        skin.draw(this.ctx, sf, this._state)
        drawStateEffects(this.ctx, sf, this._state)
        this.displayCtx.clearRect(0, 0, dsz, dsz)
        if (this._mirror) {
          this.displayCtx.save()
          this.displayCtx.scale(-1, 1)
          this.displayCtx.drawImage(this.canvas, -dsz, 0, dsz, dsz)
          this.displayCtx.restore()
        } else {
          this.displayCtx.drawImage(this.canvas, 0, 0, dsz, dsz)
        }
        // 异步缓存 ImageBitmap（不阻塞渲染）
        createImageBitmap(this.canvas).then((bmp) => {
          // 限制缓存大小（8 skins × 7 states × ~20 frames ≈ 1120 entries）
          if (this.frameCache.size > 1200) {
            // 清除一半旧缓存
            const keys = [...this.frameCache.keys()]
            for (let i = 0; i < keys.length / 2; i++) {
              this.frameCache.get(keys[i])?.close()
              this.frameCache.delete(keys[i])
            }
          }
          this.frameCache.set(cacheKey, bmp)
        })
      }

      this.animId = requestAnimationFrame(tick)
    }
    this.animId = requestAnimationFrame(tick)
  }

  destroy() {
    cancelAnimationFrame(this.animId)
    this.clearCache()
    this.displayCanvas.remove()
  }
}
