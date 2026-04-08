// Fixture: known code smells for integration testing.

class badClass {
  value: number;
}

function tooManyParams(a: number, b: number, c: number, d: number, e: number, f: number): void {
  console.log(a, b, c, d, e, f);
}

function complexFunction(n: number): number {
  if (n > 0) {
    if (n > 10) {
      if (n > 20) {
        if (n > 30) {
          if (n > 40) {
            if (n > 50) {
              if (n > 60) {
                if (n > 70) {
                  if (n > 80) {
                    if (n > 90) {
                      return 100;
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  return 0;
}
