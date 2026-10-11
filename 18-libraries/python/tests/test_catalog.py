import unittest

from helqix_drone_domain import ComponentTypeDefinition, component_type_catalog


class ComponentCatalogTests(unittest.TestCase):
    def test_vehicle_catalog_contains_universal_and_domain_types(self):
        catalog = component_type_catalog("battery electric vehicle")
        self.assertEqual(catalog["schema"], "helqix.component-type-catalog.v1")
        self.assertEqual(catalog["domain"], "vehicle")
        entries = {item["id"]: item for item in catalog["entries"]}
        self.assertIn("battery_cell", entries)
        self.assertIn("traction_inverter", entries)
        self.assertEqual(entries["battery_cell"]["supportedModels"], [{
            "modelId": "pybamm.lithium-ion.spm", "version": "Chen2020",
            "scope": "single-cell discharge only; use only when the physical cell matches the Chen2020 parameterization; not pack- or product-level",
            "requiredProperties": ["current_a", "cutoff_voltage_v"],
        }])
        self.assertEqual(entries["lithium_ion_cell"]["supportedModels"], entries["battery_cell"]["supportedModels"])
        self.assertEqual(entries["traction_inverter"]["supportedModels"], [])

    def test_lithium_ion_cell_is_explicitly_listed_for_supported_domains(self):
        for domain in ("battery", "vehicle", "drone"):
            catalog = component_type_catalog(domain)
            entries = {item["id"]: item for item in catalog["entries"]}
            with self.subTest(domain=domain):
                self.assertIn("lithium_ion_cell", entries)
                model = entries["lithium_ion_cell"]["supportedModels"][0]
                self.assertEqual(model["modelId"], "pybamm.lithium-ion.spm")
                self.assertIn("physical cell matches", model["scope"])

    def test_drone_catalog_does_not_claim_flight_model_support(self):
        catalog = component_type_catalog("Agricultural drone / UAV")
        entries = {item["id"]: item for item in catalog["entries"]}
        self.assertEqual(catalog["domain"], "drone")
        self.assertIn("propeller", entries)
        self.assertEqual(entries["propeller"]["supportedModels"], [])
        self.assertIn("require validated", entries["propeller"]["description"])

    def test_unknown_domains_still_get_generic_types(self):
        catalog = component_type_catalog("unlisted engineering system")
        self.assertEqual(catalog["domain"], "general")
        self.assertTrue(any(item["id"] == "custom_component" for item in catalog["entries"]))

    def test_catalog_definition_rejects_unversioned_or_invalid_types(self):
        with self.assertRaises(Exception):
            ComponentTypeDefinition(schema="wrong", id="motor", label="Motor", category="Actuation", domains=["general"], description="Type")
        with self.assertRaises(Exception):
            ComponentTypeDefinition(schema="helqix.component-type.v1", id="bad type", label="Motor", category="Actuation", domains=["general"], description="Type")


if __name__ == "__main__":
    unittest.main()
