from typing import Optional, Collection, Iterable
from colorama import Fore, init
from abc import ABC, abstractmethod

init(autoreset = True)


class AmbiguousData(ABC):
    @abstractmethod
    def is_ambiguous(self) -> bool:
        pass


class AmbiguousList[T](list[T], AmbiguousData):
    def __init__(self, single_data: Optional[T] = None):
        if single_data is None:
            super().__init__()
        else:
            super().__init__([single_data])

    def append(self, single_data: T) -> None:
        if single_data in self:
            return
        else:
            super().append(single_data)

    def extend(self, iterable: Iterable[T]) -> None:
        new_data = super().extend(iterable)
        super().clear
        if not new_data:
            return
        new_data = set(new_data)
        super().extend(new_data)

    def is_ambiguous(self) -> bool:
        if not self:
            return False
        elif any(isinstance(single_data, AmbiguousData) for single_data in self):
            return True
        return len(self) > 1

    def __repr__(self) -> str:
        if self.is_ambiguous():
            return Fore.RED + 'Ambiguous:' + Fore.RESET + f'{super().__repr__()}'
        elif not self:
            return f'Empty'
        else:
            return f'{self[0]}'


class AmbiguousDict[S, T](dict[S, T], AmbiguousData):
    def is_ambiguous(self) -> bool:
        if not self:
            return False
        elif any(isinstance(single_data, AmbiguousData) and single_data.is_ambiguous() for single_data in self.values()):
            return True
        else:
            return False

    def __repr__(self) -> str:
        if not self:
            return f'Empty'
        elif not self.is_ambiguous():
            return f'{super().__repr__()}'
        else:
            return Fore.RED + 'Ambiguous:' + Fore.RESET + f'{super().__repr__()}'


class AmbiguousFrozen[T](frozenset[T], AmbiguousData):
    def __new__(cls, data: Collection[T]):
        return super().__new__(cls, data)

    def is_ambiguous(self) -> bool:
        return len(self) > 1
 
    def __repr__(self) -> str:
        if self.is_ambiguous():
            return Fore.RED + 'Ambiguous:' + Fore.RESET + f'{sorted(self)}'
        else:
            return f'{next(iter(self), None)}'

    def __or__(self, other) -> 'AmbiguousFrozen[T]':
        return AmbiguousFrozen(super().__or__(other))

    def __and__(self, other) -> 'AmbiguousFrozen[T]':
        return AmbiguousFrozen(super().__and__(other))

    def __xor__(self, other) -> 'AmbiguousFrozen[T]':
        return AmbiguousFrozen(super().__xor__(other))

    def __sub__(self, other) -> 'AmbiguousFrozen[T]':
        return AmbiguousFrozen(super().__sub__(other))

