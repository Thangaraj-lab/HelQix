import helqix_circuit


def test_package_importable():
    assert helqix_circuit.__version__ == "0.1.0"
    assert helqix_circuit.PACKAGE_NAME == "helqix_circuit"
