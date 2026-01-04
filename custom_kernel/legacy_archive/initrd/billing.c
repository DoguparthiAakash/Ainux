#include <stdio.h>

int main() {
    int numItems;
    int i = 0;
    int totalBill = 0;
    int id;
    int price;
    int qty;
    int itemTotal;
    
    printf("Welcome to the Simple Billing System!\n\n");

    // Get the number of items
    printf("Enter the number of items: ");
    scanf("%d", &numItems);

    printf("\n\n------------------ Your Bill ------------------\n");
    printf("Item ID   Price     Quantity  Total\n");
    printf("------------------------------------------------\n");

    // Input details for each item and print immediately
    while (i < numItems) {
        printf("\nEnter Item %d Details:\n", i + 1);
        
        printf("ID (int): ");
        scanf("%d", &id); 
        
        printf("Price:    ");
        scanf("%d", &price);
        
        printf("Quantity: ");
        scanf("%d", &qty);
        
        itemTotal = price * qty;
        totalBill = totalBill + itemTotal;
        
        // Manual formatting with spaces
        printf("Item: %d   %d        %d         %d\n", id, price, qty, itemTotal);
        
        i = i + 1;
    }

    // Print the total bill
    printf("------------------------------------------------\n");
    printf("                  Grand Total: %d\n", totalBill);
    printf("------------------------------------------------\n");

    return 0;
}
