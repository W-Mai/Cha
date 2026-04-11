#include <string>
#include <vector>

class Animal {
    std::string name;
    int age;
public:
    Animal(std::string n, int a) : name(n), age(a) {}
    std::string getName() { return name; }
};

int factorial(int n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}
