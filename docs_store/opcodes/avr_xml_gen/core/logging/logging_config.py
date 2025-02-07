import logging
import logging.config
from .colored_formatter import ColoredFormatter


def setup_logging():
    log_config = {
        'version': 1,
        'formatters': {
            'custom': {
                '()': ColoredFormatter,
                'format': '%(asctime)s | %(levelname)s | %(name)s | %(message)s'
            },
        },
        'handlers': {
            'file': {
                'level': 'DEBUG',
                'class': 'logging.FileHandler',
                'filename': 'app.log',
                'formatter': 'custom',
            },
            'console': {
                'level': 'INFO',
                'class': 'logging.StreamHandler',
                'formatter': 'custom',
            },
        },
        'loggers': {
            '': {
                'handlers': ['file', 'console'],
                'level': 'DEBUG',
            },
        }
    }
    logging.config.dictConfig(log_config)
    logging.getLogger("pdfminer").setLevel(logging.WARNING)
