def print_board(board):
    print(f'{board[0]} | {board[1]} | {board[2]}')
    print('---------')
    print(f'{board[3]} | {board[4]} | {board[5]}')
    print('---------')
    print(f'{board[6]} | {board[7]} | {board[8]}')

def check_win(board, player):
    winning_combinations = [(0, 1, 2), (3, 4, 5), (6, 7, 8), (0, 3, 6), (1, 4, 7), (2, 5, 8), (0, 4, 8), (2, 4, 6)]
    for combination in winning_combinations:
        if board[combination[0]] == board[combination[1]] == board[combination[2]] == player:
            return True
    return False

def play_game():
    board = [' '] * 9
    current_player = 'X'

    while True:
        print_board(board)
        try:
            position = int(input(f'Player {current_player}, choose a position (1-9): ')) - 1
            if board[position] == ' ':
                board[position] = current_player
                if check_win(board, current_player):
                    print(f'Player {current_player} wins!')
                    break
                current_player = 'O' if current_player == 'X' else 'X'
            else:
                print('Invalid move, try again.')
        except (ValueError, IndexError):
            print('Invalid input, try again.')

play_game()