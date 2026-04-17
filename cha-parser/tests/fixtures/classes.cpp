#include <string>

// Normal class
class Animal {
    std::string name;
public:
    Animal(std::string n) : name(n) {}
    std::string getName() { return name; }
};

// Macro-decorated class (common in libraries like Qt, LVGL, ThorVG)
class API_EXPORT Shape {
public:
    void draw();
    int x;
    int y;
};

// Inheritance
class Dog : public Animal {
public:
    void bark() {}
};

namespace ns {
    class Inner {
        int val;
    public:
        int get() { return val; }
    };
}
