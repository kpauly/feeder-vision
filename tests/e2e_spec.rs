#[test]
#[ignore = "E2E not implemented; see specs/scenarios.md"]
fn e2e_scenario_1_empty_folder() {
    // Scenario 1: Empty Folder
    // Given an empty folder is selected
    // When the user runs "Scan"
    // Then the UI shows "No images found"
    // And CSV export is disabled
    todo!("Implement Scenario 1 E2E");
}

#[test]
#[ignore = "E2E not implemented; see specs/scenarios.md"]
fn e2e_scenario_2_present_filter_hides_empty() {
    // Scenario 2: Present filter hides empty frames
    // Given a folder with mixed empty and non-empty frames
    // When "Present only" is toggled on
    // Then only frames with birds are shown
    todo!("Implement Scenario 2 E2E");
}

#[test]
#[ignore = "E2E not implemented; see specs/scenarios.md"]
fn e2e_scenario_3_unknown_species_abstention() {
    // Scenario 3: Unknown species abstention
    // Given a crop with low similarity to any reference
    // When classified via k-NN
    // Then the label is "Unknown"
    // And confidence is below the threshold
    todo!("Implement Scenario 3 E2E");
}
