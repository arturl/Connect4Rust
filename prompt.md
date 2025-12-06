I want you to create a Rust implementation of Connect 4 game that runs online with a browswer-based UI.

For the back end game logic, use the following project: https://github.com/arturl/Connect4. You can reuse the NegaMax implementation but your
solution must be written in modern async Rust. 

Unlike https://github.com/arturl/Connect4, you will not be using Blazor. Instead, I want a clean separation of back end and front end. We will discuss 
what front end technology is most appropriate. I need the UI to be elegant and simple (with gravity + bounce effect) but let's avoid complex dependencies.

Additoinal requirements:

- The back end:
  - Must be completely stateless - it takes the board position (defined by the series of moves like B3R3B2R4), the level (1-15) and returns the AI move
  - Must use Alpha-beta pruning just like https://github.com/arturl/Connect4
  - It must be possible to set the difficulty level from 1 to 15
- Create the project file for the entire solution so I could use view and debug it in VS Code
- All project files must be located in this directory. Use Git and the appropriate .gitignore
- You must have high quality comment but make them concise. The comment must explain the dsign not merely translate what the code is doing
- We need a comprehensive set of tests that can execute against the back end (via REST API) or directly by calling the Rust game play API
- You will create a readme.md file that explains where the game logic came from and these instructions that were used to rewrite the game in Rust

If you have any clarifying questions, discuss with me before proceeding.
