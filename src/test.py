# Python script to create a 5x5 Tic Tac Toe game

def print_board(board):
    for row in board:
        print("|".join(row))
        print("-" * 10)

def check_winner(board, player):
    # Check rows
    for row in board:
        if all([cell == player for cell in row]):
            return True
    # Check columns
    for col in range(5):
        if all([board[row][col] == player for row in range(5)]):
            return True
    # Check diagonals
    if all([board[i][i] == player for i in range(5)]) or all([board[i][4-i] == player for i in range(5)]):
        return True
    return False

def is_board_full(board):
    return all([cell != ' ' for row in board for cell in row])

def main():
    board = [[' ' for _ in range(5)] for _ in range(5)]
    current_player = 'X'
    game_over = False

    while not game_over:
        print_board(board)
        row = int(input(f"Player {current_player}, enter row (0-4): "))
        col = int(input(f"Player {current_player}, enter column (0-4): "))

        if board[row][col] == ' ':
            board[row][col] = current_player
            if check_winner(board, current_player):
                print_board(board)
                print(f"Player {current_player} wins!")
                game_over = True
            elif is_board_full(board):
                print_board(board)
                print("It's a tie!")
                game_over = True
            else:
                current_player = 'O' if current_player == 'X' else 'X'
        else:
            print("That spot is already taken. Try again.")

if __name__ == "__main__":
    main()