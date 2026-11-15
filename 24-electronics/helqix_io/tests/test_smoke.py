import helqix_io


def test_package_importable():
    assert helqix_io.__version__ == "0.1.0"
    assert helqix_io.PACKAGE_NAME == "helqix_io"
