    // TODO: Should move to Fossology crate.
    /// Get scanner results and license conclusions for the files in SPDX
    /// found on the Fossology instance.
    pub fn query_fossology_for_licenses(
        &mut self,
        fossology: &Fossology,
        license_list: &LicenseList,
    ) -> Result<(), FossologyError> {
        info!("Populating SPDX from Fossology.");

        let sha256_values = self.get_unique_hashes(Algorithm::SHA256);

        // Create input for the Fossology query.
        let input: Vec<HashQueryInput> = sha256_values
            .iter()
            .map(|hash| HashQueryInput {
                sha256: Some(hash.to_string()),
                ..Default::default()
            })
            .collect();

        let response = fossology.licenses_for_hashes(&input)?;

        self.process_fossology_response(response, &license_list);

        // Add license texts to SPDX for licenses not on the SPDX license list.
        let licenses = self.get_license_ids();

        for license in licenses {
            if !license_list.includes_license(&license) {
                let spdx_license = self
                    .other_licensing_information_detected
                    .iter()
                    .find(|&lic| lic.license_identifier == license);

                match spdx_license {
                    Some(_) => {}
                    None => {
                        let license_data = fossology.license_by_short_name(&license);
                        match license_data {
                            Ok(license_data) => {
                                let license_text = if !license_data.text.is_empty() {
                                    license_data.text
                                } else {
                                    "NOASSERTION".into()
                                };
                                self.other_licensing_information_detected.push(
                                    OtherLicensingInformationDetected {
                                        license_identifier: license,
                                        extracted_text: license_text,
                                        license_name: license_data.full_name.to_string(),
                                        license_cross_reference: Vec::new(),
                                        license_comment: None,
                                    },
                                )
                            }
                            Err(_) => self.other_licensing_information_detected.push(
                                OtherLicensingInformationDetected {
                                    license_identifier: license,
                                    extracted_text: "NOASSERTION".into(),
                                    license_name: "NOASSERTION".into(),
                                    license_cross_reference: Vec::new(),
                                    license_comment: None,
                                },
                            ),
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // TODO: Should move to Fossology crate.
    /// Add information from Fossology response to the SPDX.
    fn process_fossology_response(
        &mut self,
        mut responses: Vec<HashQueryResponse>,
        license_list: &LicenseList,
    ) {
        info!("Processing Fossology response");

        // Sort response by sha256 to enable binary search.
        responses.sort_unstable_by_key(|i| i.hash.sha256.clone().unwrap().to_uppercase());

        // Loop over all the files in all packages.
        for file_information in &mut self.file_information {
            // Get sha256 of the file.
            if let Some(sha256) = file_information
                .file_checksum
                .iter()
                .find(|checksum| checksum.algorithm == Algorithm::SHA256)
            {
                // Find the corresponding item in response.
                if let Ok(response) = responses
                    .binary_search_by_key(&sha256.value.to_uppercase(), |i| {
                        i.hash.sha256.clone().unwrap().to_uppercase()
                    })
                {
                    let response = &responses[response];

                    // Add MD5 to the file in SPDX.
                    if let Some(md5) = &response.hash.md5 {
                        if file_information.checksum(Algorithm::MD5).is_none() {
                            file_information
                                .file_checksum
                                .push(Checksum::new(Algorithm::MD5, &md5))
                        }
                    }

                    // Add SHA1 to the file in SPDX.
                    if let Some(sha1) = &response.hash.sha1 {
                        if file_information.checksum(Algorithm::SHA1).is_none() {
                            file_information
                                .file_checksum
                                .push(Checksum::new(Algorithm::SHA1, &sha1))
                        }
                    }

                    // Add license findings to the file in SPDX.
                    if let Some(findings) = &response.findings {
                        // If scanner result is No_license_found and conlcusion is NOASSERTION
                        // conclude as NONE.
                        if findings.scanner.len() == 1
                            && findings.scanner.contains(&"No_license_found".to_string())
                            && findings.conclusion.len() == 1
                            && findings.conclusion.contains(&"NOASSERTION".to_string())
                        {
                            file_information.license_information_in_file =
                                license_information_to_spdx_expressions(findings.scanner.clone());
                            file_information.concluded_license = SPDXExpression("NONE".to_string());
                        } else {
                            file_information.license_information_in_file =
                                license_information_to_spdx_expressions(findings.scanner.clone());

                            if !findings.conclusion.is_empty() {
                                // TODO: Transform Fossology output to SPDX expression.
                                file_information.concluded_license =
                                    fossology_conclusions_to_spdx_expression(
                                        findings.conclusion.clone(),
                                        license_list,
                                    );
                            };
                        }

                        if !findings.copyright.is_empty() {
                            file_information.copyright_text = findings.copyright.join("\n");
                        }
                    }
                }
            }
        }
    }

// TODO: Move to Fossology crate.
/// Convert scanner hits from Fossology to vec of SPDX expressions.
fn license_information_to_spdx_expressions(license_information: Vec<String>) -> Vec<String> {
    license_information
        .into_iter()
        // Remove No_license_found
        .filter(|lic| lic != "No_license_found")
        // Remove Dual-license
        .filter(|lic| lic != "Dual-license")
        // Sanitize characters
        .map(sanitize_spdx_expression)
        // Add scanner identifier
        .map(|lic| format!("Scanner-{}", lic))
        // Add LicenseRefs
        .map(|lic| format!("LicenseRef-{}", lic))
        .collect()
}
