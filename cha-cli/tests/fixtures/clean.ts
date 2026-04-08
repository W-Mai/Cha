// Fixture: clean code with no expected findings.
export class GoodService {
    private value: number;

    constructor(value: number) {
        this.value = value;
    }

    getValue(): number {
        return this.value;
    }

    setValue(newValue: number): void {
        this.value = newValue;
    }
}
