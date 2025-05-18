// eslint.config.js or eslint.config.cjs
const tseslint = require('@typescript-eslint/eslint-plugin');
const tsParser = require('@typescript-eslint/parser');
const { FlatCompat } = require('@eslint/eslintrc');
const importPlugin = require('eslint-plugin-import');

// Initialize a compatibility object
const compat = new FlatCompat();

module.exports = [
  // Base configuration for all files
  {
    ignores: [
      'dist/',
      'node_modules/',
      'build/',
      '.eslintrc.js'
    ]
  },

  // Basic TypeScript rules (non-type-checking) for all TS files
  {
    files: ['**/*.ts', '**/*.tsx'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module'
        // Note: project is not specified here
      }
    },
    plugins: {
      '@typescript-eslint': tseslint,
      'import': importPlugin
    },
    rules: {
      // Include basic recommended rules that don't require type checking
      ...tseslint.configs.recommended.rules,
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/no-explicit-any': 'warn',

      // Use import plugin to enforce extensions
      'import/extensions': [
        'error',
        'ignorePackages',
        {
          'js': 'always',
          'ts': 'never',
          'tsx': 'never',
          'jsx': 'never'
        }
      ],

      // Override the unmound-method rule; don't consider static methods
      '@typescript-eslint/unbound-method': ['error', { "ignoreStatic": true }],
      // Ignore unused variables that start with an underscore
      "@typescript-eslint/no-unused-vars": ["error", { "argsIgnorePattern": "^_"}],
      "@typescript-eslint/no-extraneous-class": ["error", { "allowStaticOnly": true}],
    }
  },

  // Type-checking rules only for your source TypeScript files
  // (exclude config files, test fixtures, etc.)
  {
    files: ['src/**/*.ts', 'src/**/*.tsx'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        project: './tsconfig.json'
      }
    },
    plugins: {
      '@typescript-eslint': tseslint
    },
    rules: {
      // Type-checking rules
      ...tseslint.configs['recommended-type-checked'].rules,
      ...tseslint.configs['strict-type-checked'].rules
    }
  }
];