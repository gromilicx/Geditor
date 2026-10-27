from dataclasses import dataclass

@dataclass
class Person:
    name: str
    age: int


def greet(person: Person) -> str:
    return f"Hello, {person.name}!"


if __name__ == "__main__":
    print(greet(Person("Rustacean", 27)))
