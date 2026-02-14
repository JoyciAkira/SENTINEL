#[cfg(test)]
mod tests {
    use super::*;

    // Mock LLM Client for testing
    struct MockLLMClient;

    #[async_trait::async_trait]
    impl LLMClient for MockLLMClient {
        async fn generate_code(&self, _prompt: &str, _context: &llm_integration::LLMContext) -> Result<super::LLMSuggestion> {
            Ok(super::LLMSuggestion {
                id: Uuid::new_v4(),
                suggestion_type: super::LLMSuggestionType::CodeGeneration {
                    file_path: "test.rs".to_string(),
                    code: "fn test() {}".to_string(),
                    language: "rust".to_string(),
                },
                llm_name: "MockLLM".to_string(),
                content: "Generated code".to_string(),
                estimated_quality: 0.85,
                goal_alignment: 0.92,
                confidence: 0.95,
                token_cost: 150,
            })
        }
    }

    #[tokio::test]
    async fn test_llm_integration_initialization() {
        let goal_manifold = Arc::new(sentinel_core::goal_manifold::GoalManifold::new(
            sentinel_core::goal_manifold::Intent::new(
                "Test goal".to_string(),
                vec!["Test constraint".to_string()],
            )
        ));
        let alignment_field = Arc::new(sentinel_core::alignment::AlignmentField::new(goal_manifold.clone()));

        let mock_client = Arc::new(MockLLMClient);

        let manager = llm_integration::LLMIntegrationManager::new(
            goal_manifold,
            alignment_field,
            mock_client,
        )
        .await
        .expect("Failed to initialize LLM Integration Manager");

        assert_eq!(manager.get_stats().total_suggestions, 0);
        assert_eq!(manager.get_stats().avg_quality_score, 0.0);
    }

    #[tokio::test]
    async fn test_pre_validation_pass() {
        let goal_manifold = Arc::new(sentinel_core::goal_manifold::GoalManifold::new(
            sentinel_core::goal_manifold::Intent::new(
                "Test goal".to_string(),
                vec!["Test constraint".to_string()],
            )
        ));
        let alignment_field = Arc::new(sentinel_core::alignment::AlignmentField::new(goal_manifold.clone()));
        let mock_client = Arc::new(MockLLMClient);

        let mut manager = llm_integration::LLMIntegrationManager::new(
            goal_manifold,
            alignment_field,
            mock_client,
        )
        .await
        .expect("Failed to initialize LLM Integration Manager");

        let suggestion = super::LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: super::LLMSuggestionType::CodeGeneration {
                file_path: "test.rs".to_string(),
                code: "fn test() {}".to_string(),
                language: "rust".to_string(),
            },
            llm_name: "MockLLM".to_string(),
            content: "Generated code".to_string(),
            estimated_quality: 0.85,
            goal_alignment: 0.92,
            confidence: 0.95,
            token_cost: 150,
        };

        let result = manager.process_suggestion(suggestion.clone()).await;

        assert!(result.is_ok());

        let validated = result.unwrap();

        assert!(validated.passed_all_gates);
        assert!(validated.final_quality_score >= 85.0);
    }

    #[tokio::test]
    async fn test_pre_validation_fail_expensive() {
        let goal_manifold = Arc::new(sentinel_core::goal_manifold::GoalManifold::new(
            sentinel_core::goal_manifold::Intent::new(
                "Test goal".to_string(),
                vec!["Test constraint".to_string()],
            )
        ));
        let alignment_field = Arc::new(sentinel_core::alignment::AlignmentField::new(goal_manifold.clone()));
        let mock_client = Arc::new(MockLLMClient);

        let mut manager = llm_integration::LLMIntegrationManager::new(
            goal_manifold,
            alignment_field,
            mock_client,
        )
        .await
        .expect("Failed to initialize LLM Integration Manager");

        let suggestion = super::LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: super::LLMSuggestionType::CodeGeneration {
                file_path: "test.rs".to_string(),
                code: "A".to_string().repeat(5000), // Way too long
                language: "rust".to_string(),
            },
            llm_name: "MockLLM".to_string(),
            content: "Generated code".to_string(),
            estimated_quality: 0.85,
            goal_alignment: 0.92,
            confidence: 0.95,
            token_cost: 20000, // 20k tokens - too expensive
        };

        let result = manager.process_suggestion(suggestion.clone()).await;

        assert!(result.is_ok());

        let validated = result.unwrap();

        assert!(!validated.passed_all_gates);
        assert_eq!(validated.validation_results[0].explanation, "Suggestion too expensive: 20000 tokens (max 10000)");
    }

    #[tokio::test]
    async fn test_pre_validation_fail_low_confidence() {
        let goal_manifold = Arc::new(sentinel_core::goal_manifold::GoalManifold::new(
            sentinel_core::goal_manifold::Intent::new(
                "Test goal".to_string(),
                vec!["Test constraint".to_string()],
            )
        ));
        let alignment_field = Arc::new(sentinel_core::alignment::AlignmentField::new(goal_manifold.clone()));
        let mock_client = Arc::new(MockLLMClient);

        let mut manager = llm_integration::LLMIntegrationManager::new(
            goal_manifold,
            alignment_field,
            mock_client,
        )
        .await
        .expect("Failed to initialize LLM Integration Manager");

        let suggestion = super::LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: super::LLMSuggestionType::CodeGeneration {
                file_path: "test.rs".to_string(),
                code: "fn test() {}".to_string(),
                language: "rust".to_string(),
            },
            llm_name: "MockLLM".to_string(),
            content: "Generated code".to_string(),
            estimated_quality: 0.85,
            goal_alignment: 0.92,
            confidence: 0.2, // Very low confidence
            token_cost: 150,
        };

        let result = manager.process_suggestion(suggestion.clone()).await;

        assert!(result.is_ok());

        let validated = result.unwrap();

        assert!(!validated.passed_all_gates);
        assert!(validated.validation_results[0].explanation.contains("LLM confidence too low"));
    }

    #[tokio::test]
    async fn test_validate_goal_alignment() {
        let goal_manifold = Arc::new(sentinel_core::goal_manifold::GoalManifold::new(
            sentinel_core::goal_manifold::Intent::new(
                "Build authentication".to_string(),
                vec!["Use JWT".to_string(), "Secure".to_string()],
            )
        ));
        let alignment_field = Arc::new(sentinel_core::alignment::AlignmentField::new(goal_manifold.clone()));
        let mock_client = Arc::new(MockLLMClient);

        let mut manager = llm_integration::LLMIntegrationManager::new(
            goal_manifold,
            alignment_field,
            mock_client,
        )
        .await
        .expect("Failed to initialize LLM Integration Manager");

        let suggestion = super::LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: super::LLMSuggestionType::CodeGeneration {
                file_path: "auth.rs".to_string(),
                code: "fn auth() {}".to_string(),
                language: "rust".to_string(),
            },
            llm_name: "MockLLM".to_string(),
            content: "Generated auth function".to_string(),
            estimated_quality: 0.85,
            goal_alignment: 0.95, // High alignment
            confidence: 0.9,
            token_cost: 300,
        };

        let result = manager.process_suggestion(suggestion.clone()).await;

        assert!(result.is_ok());

        let validated = result.unwrap();

        assert!(validated.passed_all_gates);
        // Check that validation passed with good score
        let has_passing_score = validated.validation_results.iter().any(|v| {
            matches!(&v.result, super::ValidationStatus::Pass { score } if *score >= 85.0)
        });
        assert!(has_passing_score);
    }

    #[tokio::test]
    async fn test_validate_syntax_correctness() {
        let goal_manifold = Arc::new(sentinel_core::goal_manifold::GoalManifold::new(
            sentinel_core::goal_manifold::Intent::new(
                "Generate function".to_string(),
                vec!["Valid syntax".to_string()],
            )
        ));
        let alignment_field = Arc::new(sentinel_core::alignment::AlignmentField::new(goal_manifold.clone()));
        let mock_client = Arc::new(MockLLMClient);

        let mut manager = llm_integration::LLMIntegrationManager::new(
            goal_manifold,
            alignment_field,
            mock_client,
        )
        .await
        .expect("Failed to initialize LLM Integration Manager");

        let suggestion = super::LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: super::LLMSuggestionType::CodeGeneration {
                file_path: "valid.rs".to_string(),
                code: "fn valid() bool {}".to_string(),
                language: "rust".to_string(),
            },
            llm_name: "MockLLM".to_string(),
            content: "Generated valid function".to_string(),
            estimated_quality: 0.9,
            goal_alignment: 0.85,
            confidence: 0.95,
            token_cost: 200,
        };

        let result = manager.process_suggestion(suggestion.clone()).await;

        assert!(result.is_ok());

        let validated = result.unwrap();

        assert!(validated.passed_all_gates);
        // Check syntax validation component
        let has_syntax_check = validated
            .validation_results
            .iter()
            .any(|v| matches!(&v.component, super::ValidationComponent::SyntaxCorrectness));

        assert!(has_syntax_check);
    }

    #[tokio::test]
    async fn test_suggest_improvements() {
        let goal_manifold = Arc::new(sentinel_core::goal_manifold::GoalManifold::new(
            sentinel_core::goal_manifold::Intent::new(
                "Refactor code".to_string(),
                vec!["Clean code".to_string()],
            )
        ));
        let alignment_field = Arc::new(sentinel_core::alignment::AlignmentField::new(goal_manifold.clone()));
        let mock_client = Arc::new(MockLLMClient);

        let mut manager = llm_integration::LLMIntegrationManager::new(
            goal_manifold,
            alignment_field,
            mock_client,
        )
        .await
        .expect("Failed to initialize LLM Integration Manager");

        let suggestion = super::LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: super::LLMSuggestionType::CodeGeneration {
                file_path: "code.rs".to_string(),
                code: "fn old_code() {}".to_string(),
                language: "rust".to_string(),
            },
            llm_name: "MockLLM".to_string(),
            content: "Generated old code".to_string(),
            estimated_quality: 0.6, // Low quality
            goal_alignment: 0.7, // Low alignment
            confidence: 0.8,
            token_cost: 150,
        };

        let result = manager.process_suggestion(suggestion.clone()).await;

        assert!(result.is_ok());

        let validated = result.unwrap();

        assert!(!validated.passed_all_gates);

        // Check that improvements were suggested - final_quality_score will be low
        assert!(validated.final_quality_score < 85.0);
    }

    #[tokio::test]
    async fn test_quality_thresholds() {
        let goal_manifold = Arc::new(sentinel_core::goal_manifold::GoalManifold::new(
            sentinel_core::goal_manifold::Intent::new(
                "Test goal".to_string(),
                vec!["Test".to_string()],
            )
        ));
        let alignment_field = Arc::new(sentinel_core::alignment::AlignmentField::new(goal_manifold.clone()));
        let mock_client = Arc::new(MockLLMClient);

        let mut manager = llm_integration::LLMIntegrationManager::new(
            goal_manifold,
            alignment_field,
            mock_client,
        )
        .await
        .expect("Failed to initialize LLM Integration Manager");

        // Test default thresholds
        let default_thresholds = manager.get_quality_thresholds();

        assert_eq!(default_thresholds.min_alignment, 85.0);
        assert_eq!(default_thresholds.max_complexity, 70.0);
        assert_eq!(default_thresholds.min_test_coverage, 0.80);
        assert_eq!(default_thresholds.min_documentation, 0.85);
    }
}
