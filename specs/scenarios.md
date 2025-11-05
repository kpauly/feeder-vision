# Scenarios

Scenario 1: Empty Folder
Given an empty folder is selected
When the user runs "Scan"
Then the UI shows "No images found"
And CSV export is disabled

Scenario 2: Present filter hides empty frames
Given a folder with mixed empty and non-empty frames
When "Present only" is toggled on
Then only frames with birds are shown

Scenario 3: Unknown species abstention
Given a crop with low similarity to any reference
When classified via k-NN
Then the label is "Unknown"
And confidence is below the threshold
