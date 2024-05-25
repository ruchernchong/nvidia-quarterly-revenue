import re


def replace_text(text: str) -> str:
    # Update text to lower case
    text = text.lower()

    if text.isidentifier() or contains_single_underscore(text):
        return text

    # Remove special characters
    text = re.sub(r'[^a-zA-Z0-9_]', '_', text.lower())
    return replace_multiple_underscores(text)


def replace_multiple_underscores(text: str) -> str:
    if '__' not in text:
        return text

    return replace_multiple_underscores(text.replace('__', '_'))


def contains_single_underscore(text: str) -> bool:
    words = text.split()
    for word in words:
        if word.count('_') != 1:
            return False
    return True
