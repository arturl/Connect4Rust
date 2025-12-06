//! Connect 4 engine with alpha-beta pruning.
//! The game state is fully stateless: callers feed a move history string
//! (e.g. `B3R3B2R4`) and request a search depth (1-15). The AI plays for the
//! side whose turn is next after that history.
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const WIDTH: usize = 7;
const HEIGHT: usize = 6;
const COL_HEIGHT: usize = HEIGHT + 1; // sentinel row simplifies bit math
const MAX_CELLS: usize = WIDTH * HEIGHT;
const WIN_SCORE: i32 = 1_000_000;

/// Order legal moves so alpha-beta sees center-first branches.
const MOVE_ORDER: [usize; WIDTH] = [3, 2, 4, 1, 5, 0, 6];

/// Precomputed winning lines of four as bitmasks.
static WIN_MASKS: Lazy<Vec<u64>> = Lazy::new(generate_win_masks);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Player {
    Red,
    Blue,
}

impl Player {
    fn idx(self) -> usize {
        match self {
            Player::Red => 0,
            Player::Blue => 1,
        }
    }

    pub fn opponent(self) -> Player {
        match self {
            Player::Red => Player::Blue,
            Player::Blue => Player::Red,
        }
    }
}

#[derive(Debug, Error)]
pub enum GameError {
    #[error("invalid move string at position {position}: {reason}")]
    ParseMove { position: usize, reason: String },
    #[error("column {column} is full")]
    ColumnFull { column: usize },
    #[error("column {column} is out of bounds")]
    ColumnOutOfBounds { column: usize },
    #[error("no legal moves remain")]
    NoMoves,
    #[error("depth {0} is out of range (1-15)")]
    DepthOutOfRange(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveOutcome {
    pub player: Player,
    pub column: usize,
    pub won: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveRequest {
    pub position: String,
    pub level: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveResponse {
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameState {
    players: [u64; 2],
    heights: [u8; WIDTH],
    to_move: Player,
    moves_played: u8,
}

impl GameState {
    pub fn empty(to_move: Player) -> Self {
        Self {
            players: [0, 0],
            heights: [0; WIDTH],
            to_move,
            moves_played: 0,
        }
    }

    pub fn from_history(moves: &[TypedMove]) -> Result<Self, GameError> {
        if moves.is_empty() {
            return Ok(Self::empty(Player::Red));
        }
        let mut state = Self::empty(moves[0].player);
        for mv in moves {
            state.force_play(mv.player, mv.column)?;
        }
        state.to_move = moves
            .last()
            .map(|m| m.player.opponent())
            .unwrap_or(Player::Red);
        Ok(state)
    }

    pub fn bits(&self, player: Player) -> u64 {
        self.players[player.idx()]
    }

    pub fn legal_moves(&self) -> Vec<usize> {
        MOVE_ORDER
            .iter()
            .copied()
            .filter(|&col| (self.heights[col] as usize) < HEIGHT)
            .collect()
    }

    pub fn is_full(&self) -> bool {
        self.moves_played as usize >= MAX_CELLS
    }

    fn force_play(&mut self, player: Player, column: usize) -> Result<MoveOutcome, GameError> {
        if column >= WIDTH {
            return Err(GameError::ColumnOutOfBounds { column });
        }
        let height = self.heights[column] as usize;
        if height >= HEIGHT {
            return Err(GameError::ColumnFull { column });
        }
        let bit = 1u64 << (column * COL_HEIGHT + height);
        self.players[player.idx()] |= bit;
        self.heights[column] += 1;
        self.moves_played += 1;
        let won = has_won(self.players[player.idx()]);
        self.to_move = player.opponent();
        Ok(MoveOutcome {
            player,
            column,
            won,
        })
    }

    fn play(&mut self, column: usize) -> Result<MoveOutcome, GameError> {
        let player = self.to_move;
        self.force_play(player, column)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedMove {
    pub player: Player,
    pub column: usize,
}

pub fn parse_history(history: &str) -> Result<Vec<TypedMove>, GameError> {
    if history.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut moves = Vec::new();
    let chars: Vec<char> = history.chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        let color = chars[idx];
        let player = match color {
            'R' | 'r' => Player::Red,
            'B' | 'b' => Player::Blue,
            _ => {
                return Err(GameError::ParseMove {
                    position: idx,
                    reason: format!("expected R or B, found {color}"),
                })
            }
        };
        idx += 1;
        if idx >= chars.len() {
            return Err(GameError::ParseMove {
                position: idx,
                reason: "missing column number".to_string(),
            });
        }
        let column_char = chars[idx];
        if !column_char.is_ascii_digit() {
            return Err(GameError::ParseMove {
                position: idx,
                reason: format!("expected column digit, found {column_char}"),
            });
        }
        let column = column_char.to_digit(10).unwrap() as usize;
        if column >= WIDTH {
            return Err(GameError::ParseMove {
                position: idx,
                reason: format!("column must be 0-{}", WIDTH - 1),
            });
        }
        moves.push(TypedMove {
            player,
            column,
        });
        idx += 1;
    }
    Ok(moves)
}

pub fn best_move(request: MoveRequest) -> Result<MoveResponse, GameError> {
    if !(1..=15).contains(&request.level) {
        return Err(GameError::DepthOutOfRange(request.level));
    }
    let moves = parse_history(&request.position)?;
    let mut state = GameState::from_history(&moves)?;
    let candidate = choose_move(&mut state, request.level as usize)?;
    Ok(MoveResponse { column: candidate })
}

fn choose_move(state: &mut GameState, depth: usize) -> Result<usize, GameError> {
    let player = state.to_move;
    let mut best_col = None;
    let mut alpha = i32::MIN / 2;
    let beta = i32::MAX / 2;
    for col in state.legal_moves() {
        let mut child = state.clone();
        let outcome = child.play(col)?;
        let val = if outcome.won {
            WIN_SCORE - 1
        } else if child.is_full() {
            0
        } else {
            -negamax(
                &child,
                depth.saturating_sub(1),
                -beta,
                -alpha,
                player.opponent(),
            )
        };
        if val > alpha {
            alpha = val;
            best_col = Some(col);
        }
    }

    best_col.ok_or(GameError::NoMoves)
}

fn negamax(state: &GameState, depth: usize, mut alpha: i32, beta: i32, player: Player) -> i32 {
    if depth == 0 || state.is_full() {
        return evaluate(state, player);
    }

    let mut best = i32::MIN / 2;

    for col in state.legal_moves() {
        let mut child = state.clone();
        let outcome = child.play(col).expect("legal move must succeed");
        let score = if outcome.won {
            WIN_SCORE - 1 + depth as i32
        } else if child.is_full() {
            0
        } else {
            -negamax(&child, depth - 1, -beta, -alpha, player.opponent())
        };
        best = best.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }
    best
}

fn evaluate(state: &GameState, player: Player) -> i32 {
    let mine = state.bits(player);
    let theirs = state.bits(player.opponent());
    if has_won(mine) {
        return WIN_SCORE;
    }
    if has_won(theirs) {
        return -WIN_SCORE;
    }

    let center_bits = center_mask();
    let center_score =
        3 * (mine & center_bits).count_ones() as i32 - 3 * (theirs & center_bits).count_ones() as i32;

    let mut score = center_score;
    for mask in WIN_MASKS.iter() {
        let mine_count = (mine & mask).count_ones();
        let theirs_count = (theirs & mask).count_ones();
        if mine_count > 0 && theirs_count > 0 {
            continue; // blocked line
        }
        match (mine_count, theirs_count) {
            (3, 0) => score += 50,
            (2, 0) => score += 10,
            (1, 0) => score += 2,
            (0, 3) => score -= 50,
            (0, 2) => score -= 10,
            (0, 1) => score -= 2,
            _ => {}
        }
    }
    score
}

fn center_mask() -> u64 {
    let mut mask = 0;
    let col = WIDTH / 2;
    for row in 0..HEIGHT {
        mask |= 1u64 << (col * COL_HEIGHT + row);
    }
    mask
}

fn has_won(bits: u64) -> bool {
    // Vertical
    let mut m = bits & (bits >> 1);
    if (m & (m >> 2)) != 0 {
        return true;
    }
    // Horizontal
    m = bits & (bits >> COL_HEIGHT);
    if (m & (m >> (2 * COL_HEIGHT))) != 0 {
        return true;
    }
    // Diagonal /
    m = bits & (bits >> (COL_HEIGHT - 1));
    if (m & (m >> (2 * (COL_HEIGHT - 1)))) != 0 {
        return true;
    }
    // Diagonal \
    m = bits & (bits >> (COL_HEIGHT + 1));
    if (m & (m >> (2 * (COL_HEIGHT + 1)))) != 0 {
        return true;
    }
    false
}

fn generate_win_masks() -> Vec<u64> {
    let mut masks = Vec::new();
    // Horizontal
    for row in 0..HEIGHT {
        for col in 0..=WIDTH - 4 {
            let mut mask = 0;
            for offset in 0..4 {
                mask |= bit_for(col + offset, row);
            }
            masks.push(mask);
        }
    }
    // Vertical
    for col in 0..WIDTH {
        for row in 0..=HEIGHT - 4 {
            let mut mask = 0;
            for offset in 0..4 {
                mask |= bit_for(col, row + offset);
            }
            masks.push(mask);
        }
    }
    // Diagonal \
    for col in 0..=WIDTH - 4 {
        for row in 0..=HEIGHT - 4 {
            let mut mask = 0;
            for offset in 0..4 {
                mask |= bit_for(col + offset, row + offset);
            }
            masks.push(mask);
        }
    }
    // Diagonal /
    for col in 0..=WIDTH - 4 {
        for row in 3..HEIGHT {
            let mut mask = 0;
            for offset in 0..4 {
                mask |= bit_for(col + offset, row - offset);
            }
            masks.push(mask);
        }
    }
    masks
}

fn bit_for(col: usize, row: usize) -> u64 {
    1u64 << (col * COL_HEIGHT + row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_replay_history() {
        let history = "B2R2B1R3";
        let moves = parse_history(history).unwrap();
        let state = GameState::from_history(&moves).unwrap();
        assert_eq!(state.moves_played, 4);
        assert_eq!(state.to_move, Player::Blue);
    }

    #[test]
    fn detect_vertical_win() {
        let history = "B0R1B0R1B0R1B0";
        let moves = parse_history(history).unwrap();
        let state = GameState::from_history(&moves).unwrap();
        assert!(has_won(state.bits(Player::Blue)));
    }

    #[test]
    fn rejects_bad_depth() {
        let res = best_move(MoveRequest {
            position: "".to_string(),
            level: 0,
        });
        assert!(matches!(res, Err(GameError::DepthOutOfRange(_))));
    }

    #[test]
    fn parses_invalid_column() {
        let res = parse_history("R7");
        assert!(res.is_err());
    }

    #[test]
    fn choose_blocking_move() {
        let req = MoveRequest {
            // Red threatens a horizontal four on the bottom row; Blue must block at column 3 (0-based).
            position: "R0B0R1B1R2".to_string(),
            level: 5,
        };
        let res = best_move(req).unwrap();
        assert_eq!(res.column, 3);
    }

    #[test]
    fn wins_when_available_bottom_diagonal() {
        let res = best_move(MoveRequest {
            position: "B0R0B1R1B2R3B4R4B5R5B6R3B6R3".to_string(),
            level: 9,
        })
        .unwrap();
        assert_eq!(res.column, 3);
    }

    #[test]
    fn takes_immediate_win_horizontal() {
        // Red has three in a row on the bottom: columns 3,4,5. It must play 6 to win.
        let res = best_move(MoveRequest {
            position: "B0R3B1R4B2R5".to_string(),
            level: 8,
        })
        .unwrap();
        assert_eq!(res.column, 6);
    }

    #[test]
    fn blocks_vertical_four_incoming() {
        // Blue is threatening four in column 0 on next move; Red must play 0 to block.
        let res = best_move(MoveRequest {
            position: "B0R1B0R1B0R1".to_string(),
            level: 6,
        })
        .unwrap();
        assert_eq!(res.column, 0);
    }
}
