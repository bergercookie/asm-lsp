import logging
from colorama import Fore, init

# I wanted simple colored output for logs
# I found out answers in stack owerflow, but didn't like them
# Their Formatter modifications seemed crooked to me.
# I saw library coloredlogs, but didn't realise how to carefully use it
# So I opened logging module source code and slightly overloaded format function
class ColoredFormatter(logging.Formatter):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.FORMAT_COLORS = {
            logging.DEBUG: Fore.RESET,
            logging.INFO: Fore.BLUE,
            logging.WARNING: Fore.YELLOW,
            logging.ERROR: Fore.RED,
            logging.CRITICAL: Fore.RED,
        }

    def format(self, record) -> str:
        s = super().format(record)
        s = self.FORMAT_COLORS[record.levelno] + s + Fore.RESET
        return s

    def setLevelColor(self, levelno: int, color: str):
        if levelno not in self.FORMAT_COLORS.keys():
            raise RuntimeError(f'levelno is incorrect ({levelno})')
        self.FORMAT_COLORS[levelno] = color
