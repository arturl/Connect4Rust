import "./style.css";

const WIDTH = 7;
const HEIGHT = 6;
const CELL = 96;
const PADDING = 24;
const BUTTON_HEIGHT = 50;
const GRAVITY = 0.9;
const RESTITUTION = 0.35;

const HUMAN_COLOR = "#38bdf8"; // Blue (B)
const AI_COLOR = "#e11d48"; // Red (R)

type CellState = 0 | 1 | 2; // 0 = empty, 1 = human (blue), 2 = AI (red)

interface Animation {
  col: number;
  row: number;
  x: number;
  y: number;
  targetY: number;
  vy: number;
  color: string;
}

interface GameState {
  board: CellState[][];
  history: string;
  level: number;
  busy: boolean;
}

interface WinningLine {
  player: CellState;
  cells: Array<{ col: number; row: number }>;
}

const canvas = document.createElement("canvas");
const ctx = canvas.getContext("2d")!;
canvas.width = WIDTH * CELL + PADDING * 2;
canvas.height = BUTTON_HEIGHT + PADDING + HEIGHT * CELL + PADDING * 2;

const app = document.querySelector<HTMLDivElement>("#app")!;
const header = document.createElement("header");
header.innerHTML = `
  <div class="title">
    <div class="eyebrow">Rust + Canvas</div>
    <h1>Connect 4</h1>
  </div>
  <div class="controls">
    <label>
      Depth
      <input id="level" type="range" min="1" max="15" value="7" />
      <span id="levelValue">7</span>
    </label>
    <button id="reset">Reset</button>
  </div>
`;

const status = document.createElement("div");
status.className = "status";
status.textContent = "You are Blue. Choose a column.";

const infoPanel = document.createElement("div");
infoPanel.className = "info-panel";
const traceLine = document.createElement("div");
traceLine.className = "info-row";
const traceLabel = document.createElement("span");
traceLabel.textContent = "Trace:";
const traceValue = document.createElement("code");
traceValue.id = "traceValue";
traceValue.textContent = "(empty)";
traceLine.append(traceLabel, traceValue);

const scoreLine = document.createElement("div");
scoreLine.className = "info-row";
const scoreLabel = document.createElement("span");
scoreLabel.textContent = "Outcome:";
const scoreValue = document.createElement("span");
scoreValue.id = "outcomeValue";
scoreValue.textContent = "In play";
scoreLine.append(scoreLabel, scoreValue);

infoPanel.append(traceLine, scoreLine);

app.append(header, canvas, status, infoPanel);

const state: GameState = {
  board: Array.from({ length: HEIGHT }, () => Array<CellState>(WIDTH).fill(0)),
  history: "",
  level: 7,
  busy: false,
};

const animations: Animation[] = [];
let winLine: WinningLine | null = null;
let pulseTime = 0;

const levelInput = document.querySelector<HTMLInputElement>("#level")!;
const levelValue = document.querySelector<HTMLSpanElement>("#levelValue")!;
const resetBtn = document.querySelector<HTMLButtonElement>("#reset")!;

levelInput.addEventListener("input", () => {
  state.level = Number(levelInput.value);
  levelValue.textContent = levelInput.value;
});

resetBtn.addEventListener("click", () => {
  resetGame();
});

canvas.addEventListener("click", async (ev) => {
  const rect = canvas.getBoundingClientRect();
  const clickX = ev.clientX - rect.left;
  const clickY = ev.clientY - rect.top;

  // Scale coordinates from display size to canvas size
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;
  const x = clickX * scaleX;
  const y = clickY * scaleY;

  // Only handle clicks in the button row area
  if (y < PADDING / 2 || y > BUTTON_HEIGHT + PADDING / 2) return;

  const col = Math.floor((x - PADDING) / CELL);
  if (col < 0 || col >= WIDTH) return;

  await handleMove(col);
});

function resetGame() {
  state.board = Array.from({ length: HEIGHT }, () => Array<CellState>(WIDTH).fill(0));
  state.history = "";
  state.busy = false;
  animations.splice(0, animations.length);
  winLine = null;
  status.textContent = "You are Blue. Choose a column.";
  updateInfo();
  draw();
}

function appendHistory(player: CellState, col: number) {
  const code = player === 1 ? "B" : "R";
  state.history += `${code}${col}`;
  updateInfo();
}

function dropPiece(player: CellState, col: number): number {
  for (let row = 0; row < HEIGHT; row++) {
    if (state.board[row][col] === 0) {
      state.board[row][col] = player;
      return row;
    }
  }
  return -1;
}

async function aiMove() {
  try {
    const url = `/api/move?position=${encodeURIComponent(
      state.history
    )}&level=${state.level}`;
    const res = await fetch(url, { method: "GET", cache: "no-store" });
    if (!res.ok) {
      throw new Error(await res.text());
    }
    const body = (await res.json()) as { column: number };
    const row = dropPiece(2, body.column);
    if (row === -1) {
      throw new Error("AI attempted an illegal column");
    }
    appendHistory(2, body.column);
    spawnAnimation(body.column, row, AI_COLOR);
  } catch (err) {
    status.textContent = `API error: ${(err as Error).message}`;
  }
  draw();
}

async function handleMove(col: number) {
  if (state.busy) return;
  state.busy = true;
  const row = dropPiece(1, col);
  if (row === -1) {
    state.busy = false;
    return;
  }
  appendHistory(1, col);
  spawnAnimation(col, row, HUMAN_COLOR);
  draw();
  await waitForAnimations();
  status.textContent = "AI (Red) thinking...";
  await aiMove();
  await waitForAnimations();
  state.busy = false;
  status.textContent = "AI done. Your turn.";
  updateInfo();
}

function spawnAnimation(col: number, row: number, color: string) {
  const x = PADDING + col * CELL + CELL / 2;
  const boardTop = BUTTON_HEIGHT + PADDING;
  const targetY = boardTop + PADDING + (HEIGHT - 1 - row) * CELL + CELL / 2;
  animations.push({
    col,
    row,
    x,
    y: boardTop - CELL,
    targetY,
    vy: 0,
    color,
  });
}

function update() {
  const now = performance.now();
  let last = now;
  const tick = () => {
    const current = performance.now();
    const dt = (current - last) / 16; // approx 60fps scalar
    last = current;
    pulseTime = current;
    stepAnimations(dt);
    draw();
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

function waitForAnimations(): Promise<void> {
  return new Promise((resolve) => {
    const check = () => {
      if (animations.length === 0) {
        resolve();
      } else {
        requestAnimationFrame(check);
      }
    };
    check();
  });
}

function stepAnimations(dt: number) {
  animations.forEach((anim) => {
    anim.vy += GRAVITY * dt;
    anim.y += anim.vy;
    if (anim.y >= anim.targetY) {
      anim.y = anim.targetY;
      anim.vy = -anim.vy * RESTITUTION;
      if (Math.abs(anim.vy) < 0.8) {
        anim.vy = 0;
      }
    }
  });
  for (let i = animations.length - 1; i >= 0; i--) {
    if (animations[i].vy === 0 && animations[i].y === animations[i].targetY) {
      animations.splice(i, 1);
    }
  }
}

function draw() {
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  const gradient = ctx.createLinearGradient(0, 0, canvas.width, canvas.height);
  gradient.addColorStop(0, "#0f172a");
  gradient.addColorStop(1, "#0b1221");
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  drawButtons();
  drawBoard();
  drawPieces();
  drawAnimations();
}

function updateInfo() {
  const traceValue = document.querySelector<HTMLElement>("#traceValue");
  const outcomeValue = document.querySelector<HTMLElement>("#outcomeValue");
  if (traceValue) {
    traceValue.textContent = state.history.length === 0 ? "(empty)" : state.history;
  }
  winLine = findWinningLine(1) ?? findWinningLine(2);
  if (outcomeValue) {
    outcomeValue.textContent = describeOutcome();
  }
}

function describeOutcome(): string {
  const humanWin = findWinningLine(1) !== null;
  const aiWin = findWinningLine(2) !== null;
  if (humanWin && aiWin) return "Conflict state";
  if (humanWin) return "You win!";
  if (aiWin) return "AI wins!";
  if (state.busy) return "AI thinking...";
  return "AI done. Your turn.";
}

function findWinningLine(player: CellState): WinningLine | null {
  for (let c = 0; c < WIDTH; c++) {
    for (let r = 0; r < HEIGHT; r++) {
      const dirs = [
        [1, 0],
        [0, 1],
        [1, 1],
        [1, -1],
      ] as const;
      for (const [dc, dr] of dirs) {
        const cells = collectLine(player, c, r, dc, dr);
        if (cells) {
          return { player, cells };
        }
      }
    }
  }
  return null;
}

function collectLine(
  player: CellState,
  col: number,
  row: number,
  dc: number,
  dr: number
): Array<{ col: number; row: number }> | null {
  const cells: Array<{ col: number; row: number }> = [];
  for (let i = 0; i < 4; i++) {
    const c = col + dc * i;
    const r = row + dr * i;
    if (c < 0 || c >= WIDTH || r < 0 || r >= HEIGHT) return null;
    if (state.board[r][c] !== player) return null;
    cells.push({ col: c, row: r });
  }
  return cells.length === 4 ? cells : null;
}

function drawButtons() {
  for (let col = 0; col < WIDTH; col++) {
    const x = PADDING + col * CELL;
    const y = PADDING / 2;
    const w = CELL - 12;
    const h = BUTTON_HEIGHT - 10;

    // Button background
    ctx.fillStyle = "rgba(56, 189, 248, 0.1)";
    ctx.strokeStyle = "rgba(56, 189, 248, 0.4)";
    ctx.lineWidth = 1;
    roundRect(x + 6, y, w, h, 12);
    ctx.fill();
    ctx.stroke();

    // Button text
    ctx.fillStyle = "#e5e7eb";
    ctx.font = "700 16px Space Grotesk, sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(`${col}`, x + CELL / 2, y + h / 2);
  }
}

function drawBoard() {
  const boardTop = BUTTON_HEIGHT + PADDING;
  ctx.fillStyle = "#111827";
  const boardX = PADDING / 2;
  const boardY = boardTop + PADDING / 2;
  const boardW = WIDTH * CELL + PADDING;
  const boardH = HEIGHT * CELL + PADDING;
  roundRect(boardX, boardY, boardW, boardH, 18);
  ctx.fill();

  ctx.fillStyle = "#1f2937";
  for (let col = 0; col < WIDTH; col++) {
    for (let row = 0; row < HEIGHT; row++) {
      const x = PADDING + col * CELL + CELL / 2;
      const y = boardTop + PADDING + (HEIGHT - 1 - row) * CELL + CELL / 2;
      ctx.beginPath();
      ctx.arc(x, y, CELL * 0.35, 0, Math.PI * 2);
      ctx.fill();
    }
  }
}

function drawPieces() {
  for (let col = 0; col < WIDTH; col++) {
    for (let row = 0; row < HEIGHT; row++) {
      const cell = state.board[row][col];
      if (cell === 0) continue;
      // Avoid drawing a static disc while its animation is still falling.
      if (animations.some((anim) => anim.col === col && anim.row === row)) {
        continue;
      }
      const color = cell === 1 ? HUMAN_COLOR : AI_COLOR;
      const isWinner =
        winLine?.cells.some((c) => c.col === col && c.row === row) ?? false;
      drawDisc(col, row, color, isWinner);
    }
  }
}

function drawAnimations() {
  animations.forEach((anim) => {
    drawDiscAt(anim.x, anim.y, anim.color, false);
  });
}

function drawDisc(col: number, row: number, color: string, pulse: boolean) {
  const boardTop = BUTTON_HEIGHT + PADDING;
  const x = PADDING + col * CELL + CELL / 2;
  const y = boardTop + PADDING + (HEIGHT - 1 - row) * CELL + CELL / 2;
  drawDiscAt(x, y, color, pulse);
}

function drawDiscAt(x: number, y: number, color: string, pulse: boolean) {
  const scale = pulse ? 1 + 0.08 * Math.sin(pulseTime / 150) : 1;
  ctx.beginPath();
  ctx.arc(x, y, CELL * 0.35 * scale, 0, Math.PI * 2);
  const rad = ctx.createRadialGradient(
    x - 10,
    y - 10,
    CELL * 0.1 * scale,
    x,
    y,
    CELL * 0.4 * scale
  );
  rad.addColorStop(0, color);
  rad.addColorStop(1, shade(color, -20));
  ctx.fillStyle = rad;
  ctx.fill();
}

function shade(color: string, amount: number): string {
  const num = parseInt(color.replace("#", ""), 16);
  const r = clamp(((num >> 16) & 0xff) + amount, 0, 255);
  const g = clamp(((num >> 8) & 0xff) + amount, 0, 255);
  const b = clamp((num & 0xff) + amount, 0, 255);
  return `rgb(${r}, ${g}, ${b})`;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function roundRect(x: number, y: number, w: number, h: number, r: number) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

draw();
update();
