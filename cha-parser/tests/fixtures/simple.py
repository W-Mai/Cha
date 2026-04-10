"""Module docstring."""

import os
import sys
from pathlib import Path
from typing import List, Optional


def simple_function(x, y):
    return x + y


def complex_function(data: List[int], threshold: int = 10) -> Optional[int]:
    """Find first value above threshold."""
    result = None
    for item in data:
        if item > threshold:
            result = item
            break
        elif item == threshold:
            # exact match
            result = item
    return result


class Animal:
    def __init__(self, name: str, sound: str):
        self.name = name
        self.sound = sound
        self.listeners = []

    def speak(self) -> str:
        return f"{self.name} says {self.sound}"

    def add_listener(self, listener):
        self.listeners.append(listener)

    def notify(self):
        for listener in self.listeners:
            listener(self)


class Dog(Animal):
    def __init__(self, name: str):
        super().__init__(name, "Woof")

    def speak(self) -> str:
        return f"{self.name} barks!"

    def fetch(self, item: str) -> str:
        return f"{self.name} fetches {item}"


class EmptyInterface:
    """Abstract base."""
    def do_something(self):
        ...

    def do_other(self):
        pass


def long_chain_example(obj):
    return obj.a.b.c.d.e


def delegating(service):
    return service.handler.process()
