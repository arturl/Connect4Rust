# Connect 4 in Async Rust + Canvas UI

Stateless Connect 4 engine and HTTP API built with modern async Rust (`axum` + `tokio`), paired with a minimal browser UI (Vite + TypeScript + Canvas). The game logic is adapted from the alpha-beta / negamax approach described in [arturl/Connect4](https://github.com/arturl/Connect4), rewritten as a standalone Rust library.

## Project layout
- `connect4/`: Pure game engine (bitboard representation, alpha-beta negamax with move ordering, difficulty 1–15 maps to search depth).
- `server/`: HTTP layer exposing a stateless GET API and serving the built web assets.
- `web/`: Vite + TypeScript + Canvas frontend with a simple gravity/bounce animation and zero heavy frameworks.
- `.vscode/`: Launch + tasks to debug and build in VS Code.

## API
`GET /api/move?position=B3R3B2R4&level=8`
- `position`: Move history as alternating tokens like `B3R3B2R4` (`B` = Blue, `R` = Red, columns are 1–7). The next move is inferred from the parity of that string.
- `level`: Search depth (1–15). Higher numbers play stronger but take longer.
- Response: `{ "column": 3 }` (zero-based column index).
- Caching: Responses are safe to cache but the server ships `Cache-Control: no-store` on the frontend requests.

## Running
Back end:
```bash
cargo run -p server
```
Frontend (dev):
```bash
cd web
npm install
npm run dev
```
Frontend (build to `web/dist`, served by the server binary):
```bash
cd web
npm install
npm run build
```

## Testing
- Engine tests: `cargo test -p connect4`
- API tests: `cargo test -p server`
- End-to-end (manual): run the server, then open the Vite dev server (or the built app) and play.

## Design notes
- Statelessness: the API never keeps session; callers send the full move history and desired depth.
- Engine: compact bitboard layout with a sentinel row, precomputed winning masks, center-first move ordering, and a heuristic that rewards open threes/twos. Depth directly equals difficulty.
- Frontend: vanilla TS + Canvas for simplicity; gravity/bounce animation is a lightweight physics loop (no external graphics libs).
- Separation: backend and frontend are independent; the server nests `/api` and can serve the built `web/dist`.

## Extending
- Tweak the heuristic weights in `connect4/src/lib.rs` to adjust playing style.
- Add more API metadata (scores, PV) by extending `MoveResponse`.
- Swap out the frontend if you prefer a framework; the API stays the same.
