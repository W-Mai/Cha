#include <stdio.h>
#include <stdlib.h>

typedef struct {
    int x;
    int y;
} Point;

struct Color {
    int r;
    int g;
    int b;
};

int add(int a, int b) {
    return a + b;
}

void greet(const char* name) {
    printf("Hello, %s!\n", name);
}
