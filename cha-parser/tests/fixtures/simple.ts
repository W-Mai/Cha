import { readFile } from "fs";
import path from "path";

function greet(name: string): string {
    return `Hello, ${name}`;
}

class Greeter {
    name: string;

    constructor(name: string) {
        this.name = name;
    }

    greet(): string {
        return `Hello, ${this.name}`;
    }
}
