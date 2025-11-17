# Claude Code Skills

This directory contains reusable skills for working with the NVIDIA Quarterly Revenue Analysis project.

## Available Skills

### analyze
Run the complete NVIDIA quarterly revenue analysis pipeline on the latest PDF. This skill automatically detects the latest PDF, extracts data, calculates growth rates, and generates visualisations.

**Usage:** Invoke when you want to run the full analysis.

### test-and-format
Run the complete test and code quality suite including pytest, Black formatter, and pre-commit hooks. Use this before committing changes to ensure code quality.

**Usage:** Invoke before committing code changes.

### setup-project
Perform initial project setup including dependency installation and pre-commit hook configuration. Use this when first cloning the repository.

**Usage:** Invoke during initial setup or after major dependency changes.

### add-new-quarter
Guide through adding a new quarterly revenue PDF to the analysis. Helps with file naming, placement, and verification of data extraction.

**Usage:** Invoke when you have a new quarterly PDF to add to the analysis.

## How Skills Work

Skills are prompts that can be invoked in Claude Code using the Skill tool. They provide specialized workflows for common tasks in this project.

## Creating New Skills

To create a new skill:
1. Create a new `.md` file in this directory
2. Write a clear description of what the skill does
3. Commit the file to the repository
4. The skill will be automatically available in Claude Code
