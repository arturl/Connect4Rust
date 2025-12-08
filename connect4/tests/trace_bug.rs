use connect4::{best_move, parse_history, GameState, MoveRequest};

#[test]
fn test_failing_trace() {
    // The trace where AI failed to find the winning move R1
    let trace = "B3R3B2R4B3R3B3R4B2R2B1R0B5";

    println!("\n=== Analyzing trace: {} ===", trace);

    let moves = parse_history(trace).unwrap();
    let state = GameState::from_history(&moves).unwrap();

    println!("\nCurrent board:");
    state.print_board();

    println!("\nTesting each possible Red move:");
    for col in state.legal_moves() {
        let test_trace = format!("{}R{}", trace, col);
        let test_moves = parse_history(&test_trace).unwrap();
        let test_state = GameState::from_history(&test_moves).unwrap();

        // Can't check win directly, but we can see if adding the move to history changes things
        println!("  R{}: (testing...)", col);
    }

    // Ask the AI for best move
    println!("\nAsking AI for best move at level 7...");
    let response = best_move(MoveRequest {
        position: trace.to_string(),
        level: 7,
    }).unwrap();

    println!("AI chose column: {}", response.column);
    println!("\nExpected: Column 1 (should win immediately)");
    println!("Actual:   Column {}", response.column);

    // The AI should choose column 1 which wins immediately
    assert_eq!(response.column, 1, "AI should choose column 1 for immediate win!");
}
