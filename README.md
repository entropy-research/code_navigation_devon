Sure, here's a README file for your Python package:

```markdown
# Code Navigation Python Package

This Python package provides functionality for indexing and searching code repositories using Rust and PyO3. The package supports text search, fuzzy search, and retrieving token information and hoverable ranges from the code.

## Installation

To install the package, use the following command:

```sh
pip install code_nav_devon
```

## Usage

### Import the Package

```python
import code_nav_devon
```

### Functions

#### `go_to`

Retrieves token information for a given position in a file.

##### Parameters
- `root_path_str` (str): The root path of the repository.
- `index_path_str` (str): The path where the index is stored.
- `relative_path` (str): The relative path of the file.
- `line` (int): The line number.
- `start_index` (int): The start index in the line.
- `end_index` (int): The end index in the line.

##### Returns
- `str`: Token information.

##### Example

```python
result = code_nav_devon.go_to("/path/to/repo", "/path/to/index", "src/main.rs", 10, 0, 5)
print(result)
```

#### `text_search`

Performs a text search in the code repository.

##### Parameters
- `root_path_str` (str): The root path of the repository.
- `index_path_str` (str): The path where the index is stored.
- `query` (str): The search query.
- `case_sensitive` (bool): Whether the search should be case sensitive.

##### Returns
- `str`: Search results.

##### Example

```python
result = code_nav_devon.text_search("/path/to/repo", "/path/to/index", "search term", True)
print(result)
```

#### `get_hoverable_ranges`

Retrieves the hoverable ranges for a given file.

##### Parameters
- `root_path_str` (str): The root path of the repository.
- `index_path_str` (str): The path where the index is stored.
- `relative_path` (str): The relative path of the file.

##### Returns
- `str`: Hoverable ranges in JSON format.

##### Example

```python
result = code_nav_devon.get_hoverable_ranges("/path/to/repo", "/path/to/index", "src/main.rs")
print(result)
```

## License

This project is licensed under the MIT License.
```

### Notes:
1. This README assumes the package name is `code_nav_devon` as per your `lib.rs` module name. If your package name is different, make sure to replace it accordingly.
2. The usage examples should be adjusted based on the actual paths and use cases.
3. Ensure that your package is properly configured for distribution if you plan to publish it to PyPI or any other repository.