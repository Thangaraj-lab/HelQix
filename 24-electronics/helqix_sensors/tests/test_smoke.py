import helqix_sensors


def test_package_importable():
    assert helqix_sensors.__version__ == "0.1.0"
    assert helqix_sensors.PACKAGE_NAME == "helqix_sensors"
