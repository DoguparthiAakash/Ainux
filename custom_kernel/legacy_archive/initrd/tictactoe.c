#include <stdio.h>

int main() {
    int b0 = ' ', b1 = ' ', b2 = ' ';
    int b3 = ' ', b4 = ' ', b5 = ' ';
    int b6 = ' ', b7 = ' ', b8 = ' ';

    int turn = 0;   // 0 = X, 1 = O
    int moves = 0;
    int playing = 1;

    printf("Tic-Tac-Toe (C Version)\n");
    printf("Enter 0-8 to play.\n");

    while (playing) {
        // Print board
        printf("\n %c | %c | %c \n", b0, b1, b2);
        printf("-----------\n");
        printf(" %c | %c | %c \n", b3, b4, b5);
        printf("-----------\n");
        printf(" %c | %c | %c \n", b6, b7, b8);

        int cur = 'O';
        if (turn == 0) cur = 'X';

        printf("\nPlayer %c move (0-8): ", cur);

        int slot;
        scanf("%d", &slot);

        int valid = 0;

        if (slot == 0 && b0 == ' ') { b0 = cur; valid = 1; }
        if (slot == 1 && b1 == ' ') { b1 = cur; valid = 1; }
        if (slot == 2 && b2 == ' ') { b2 = cur; valid = 1; }
        if (slot == 3 && b3 == ' ') { b3 = cur; valid = 1; }
        if (slot == 4 && b4 == ' ') { b4 = cur; valid = 1; }
        if (slot == 5 && b5 == ' ') { b5 = cur; valid = 1; }
        if (slot == 6 && b6 == ' ') { b6 = cur; valid = 1; }
        if (slot == 7 && b7 == ' ') { b7 = cur; valid = 1; }
        if (slot == 8 && b8 == ' ') { b8 = cur; valid = 1; }

        if (!valid) {
            printf("Invalid move.\n");
            continue;
        }

        moves = moves + 1;

        // Check win
        if (
            (b0 == cur && b1 == cur && b2 == cur) ||
            (b3 == cur && b4 == cur && b5 == cur) ||
            (b6 == cur && b7 == cur && b8 == cur) ||
            (b0 == cur && b3 == cur && b6 == cur) ||
            (b1 == cur && b4 == cur && b7 == cur) ||
            (b2 == cur && b5 == cur && b8 == cur) ||
            (b0 == cur && b4 == cur && b8 == cur) ||
            (b2 == cur && b4 == cur && b6 == cur)
        ) {
            printf("\nPlayer %c wins!\n", cur);
            playing = 0;
        }

        // Check draw
        if (playing && moves == 9) {
            printf("\nDraw game!\n");
            playing = 0;
        }

        // Switch player
        if (playing) turn = !turn;
    }

    return 0;
}
