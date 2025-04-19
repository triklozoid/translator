# Clipboard Translator

A lightweight, intelligent clipboard translation tool that automatically detects the source language and selects the appropriate target language based on your preferences.

## Features

- **Automatic Language Detection**: Uses the [lingua](https://github.com/pemistahl/lingua-rs) library to detect the source language of clipboard text
- **Smart Language Selection**: Intelligently chooses the target language based on your primary and secondary language preferences
- **Configurable**: Easily customize your language preferences and translation service settings
- **One-Click Copy & Close**: Translate and copy with minimal interruption to your workflow
- **OpenAI/OpenRouter Integration**: High-quality translations powered by AI language models

## How It Works

The application uses a smart algorithm to determine the target language:

```
// Variables:
// PRIMARY_LANGUAGE   — user's primary language
// SECONDARY_LANGUAGE — second language (most common translation from PRIMARY_LANGUAGE)
// LAST_LANGUAGE      — last selected target language (or null)
// SRC                — language of the source text

function chooseTargetLanguage(SRC, PRIMARY_LANGUAGE, SECONDARY_LANGUAGE, LAST_LANGUAGE):
    // 1. If the source isn't the primary language, translate into the primary language
    if SRC ≠ PRIMARY_LANGUAGE:
        return PRIMARY_LANGUAGE

    // 2. If the source is the primary language and there's a meaningful last choice, use it
    if LAST_LANGUAGE ≠ null AND LAST_LANGUAGE ≠ PRIMARY_LANGUAGE:
        return LAST_LANGUAGE

    // 3. Otherwise, fall back to the secondary language
    return SECONDARY_LANGUAGE
```

## Installation

### Prerequisites

- Rust and Cargo
- GTK3 development libraries
- OpenRouter API key (or OpenAI API key)

### Building from Source

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/clipboard-translator.git
   cd clipboard-translator
   ```

2. Build the application:
   ```bash
   cargo build --release
   ```

3. Set up your API key:
   ```bash
   export OPENROUTER_API_KEY=your_api_key_here
   ```

## Configuration

The application creates a configuration file at `~/.config/translator/config.toml` with the following settings:

```toml
api_url = "https://openrouter.ai/api/v1"
model_version = "openai/gpt-4o"
primary_language = "RU"
secondary_language = "EN"
all_target_languages = ["EN", "RU", "PT", "UK", "DE", "FR", "ES", "IT", "PL"]
```

- `primary_language`: Your main language (default: Russian)
- `secondary_language`: Your second most used language (default: English)
- `all_target_languages`: List of languages available in the UI
- `api_url`: API endpoint for translations
- `model_version`: AI model to use for translations

## Usage

1. Copy text in any language to your clipboard
2. Run the application:
   ```bash
   ./run
   ```
3. The application will automatically detect the source language and translate to the appropriate target language
4. Click on any language button to translate to that specific language
5. Click "Copy & Close" to copy the translation to your clipboard and close the application

## License

[MIT License](LICENSE)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
