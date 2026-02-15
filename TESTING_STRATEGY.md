# Comprehensive Testing Strategy for flat-cli v0.5.0

## Testing Philosophy
- **Quality over quantity**: Each test validates real use cases
- **Edge case coverage**: Test boundary conditions and error scenarios
- **Integration focus**: Test feature interactions, not just individual functions
- **Real-world scenarios**: Use realistic repository structures and data
- **Performance awareness**: Validate performance characteristics

## Test Categories

### 1. Core Functionality (Existing - Maintain)
- ✅ Basic flatten operations
- ✅ File filtering (include/exclude)
- ✅ Compression mode
- ✅ Token budgeting
- ✅ Dry-run mode

### 2. NEW: Edge Cases & Boundary Conditions
- Empty directories
- Single file repositories
- Deeply nested directories (>20 levels)
- Very large files (>1MB)
- Unusual file names (unicode, spaces, special chars)
- Symbolic links and cyclic references
- Mixed line endings (CRLF, LF, CR)
- Binary vs text file detection edge cases

### 3. NEW: Format & Output Features
- JSON format edge cases (special chars, large files)
- Template rendering with missing variables
- Template with special characters in data
- Output to files with various permissions
- stdout vs file output consistency

### 4. NEW: GitHub Integration
- URL parsing edge cases
- Branch/subpath handling
- Private repo simulation
- Rate limiting scenarios
- Network error handling

### 5. NEW: MCP Server Mode
- JSON-RPC protocol compliance
- Concurrent request handling
- Error response format validation
- Tool parameter validation
- Server lifecycle

### 6. NEW: Integration Scenarios
- Compression + Templates
- GitHub URLs + Token budget
- Multiple filters + JSON format
- Gitignore edge cases

### 7. NEW: Error Handling & Recovery
- Invalid paths
- Permission denied scenarios
- Corrupted files
- Out of disk space
- Invalid configurations
- Network timeouts

### 8. NEW: Performance & Scalability
- Large repositories (1000+ files)
- Large files (10MB+)
- Deeply nested structures
- Token budget edge cases
- Memory usage validation

### 9. NEW: Security
- Path traversal prevention
- Secret file detection edge cases
- Binary file detection edge cases
- Gitignore interpretation correctness

### 10. NEW: Real Repository Testing
- Actual open source repos analysis
- Multi-language projects
- Build artifacts handling
- Node_modules exclusion
- Git internals handling

## Test Data Strategy
- Use realistic repository structures
- Include multiple programming languages
- Test with actual .gitignore patterns
- Cover common build artifacts
- Include edge case file names

## Coverage Metrics
- Target: >80% code coverage
- Focus on error paths and edge cases
- Validate all CLI flags individually and in combinations
