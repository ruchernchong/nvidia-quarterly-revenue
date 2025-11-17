from src.utils.replace_text import (
    replace_text,
    contains_single_underscore,
    replace_multiple_underscores,
)


def test_replace_spaces():
    assert replace_text("Data Center") == "data_center"


def test_single_underscore():
    assert contains_single_underscore("Data_Visualisation") == True
    assert replace_text("Data_Visualisation") == "data_visualisation"


def test_replace_special_characters():
    assert replace_text("OEM & Other") == "oem_other"


def test_replace_multiple_underscores():
    assert replace_multiple_underscores("Hello___World") == "Hello_World"
