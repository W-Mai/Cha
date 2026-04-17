#pragma once

class Widget {
public:
    virtual void draw() = 0;
    int width;
    int height;
};

struct Point {
    float x;
    float y;
};
