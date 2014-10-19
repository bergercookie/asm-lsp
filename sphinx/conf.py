import sphinx_bootstrap_theme

extensions = [
    'sphinx.ext.autodoc',
    'sphinx.ext.viewcode',
]

source_suffix = '.rst'
master_doc = 'index'

project = u'CPU Opcodes'
copyright = u'2014, Georgia Institute of Technology'

version = '0.1.0'
release = '0.1.0'

pygments_style = 'sphinx'

html_theme = 'bootstrap'
html_theme_path = sphinx_bootstrap_theme.get_html_theme_path()
