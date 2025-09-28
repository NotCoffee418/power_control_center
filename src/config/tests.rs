use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_from_json_str() {
        let json_str = r#"
        {
            "database_path": "../home-control-center-dev.db",
            "listen_address": "127.0.0.1",
            "listen_port": 9040,
            "smart_meter_api_endpoint": "http://raspberrypi.local:9039",
            "ac_controller_endpoints": {
                "LivingRoom": {
                    "endpoint": "http://192.168.50.201",
                    "api_key": "secret123"
                },
                "Veranda": {
                    "endpoint": "http://192.168.50.202",
                    "api_key": "secret456"
                }
            }
        }
        "#;

        let config = get_config_from_json_str(json_str);

        assert_eq!(config.database_path, "../home-control-center-dev.db");
        assert_eq!(config.listen_address, "127.0.0.1");
        assert_eq!(config.listen_port, 9040);
        assert_eq!(
            config.smart_meter_api_endpoint,
            "http://raspberrypi.local:9039"
        );

        // Test AC controller endpoints
        assert_eq!(config.ac_controller_endpoints.len(), 2);

        let living_room = config.ac_controller_endpoints.get("LivingRoom").unwrap();
        assert_eq!(living_room.endpoint, "http://192.168.50.201");
        assert_eq!(living_room.api_key, "secret123".to_string());

        let veranda = config.ac_controller_endpoints.get("Veranda").unwrap();
        assert_eq!(veranda.endpoint, "http://192.168.50.202");
        assert_eq!(veranda.api_key, "secret456");
    }

    #[test]
    #[should_panic(expected = "Failed to parse config JSON")]
    fn test_get_config_from_invalid_json() {
        let invalid_json = r#"{ "invalid": json structure }"#;
        get_config_from_json_str(invalid_json);
    }

    #[test]
    #[should_panic(expected = "Failed to parse config JSON")]
    fn test_config_missing_api_key_should_crash() {
        let json_str = r#"
    {
        "database_path": "../test.db",
        "listen_address": "0.0.0.0",
        "listen_port": 8080,
        "smart_meter_api_endpoint": "http://localhost:9000",
        "ac_controller_endpoints": {
            "TestRoom": {
                "endpoint": "http://192.168.1.100"
            }
        }
    }
    "#;

        get_config_from_json_str(json_str); // This should panic
    }
}
