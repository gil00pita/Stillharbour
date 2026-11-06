import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import hooks from 'eslint-plugin-react-hooks';
import refresh from 'eslint-plugin-react-refresh';
import globals from 'globals';

export default tseslint.config(
  {
    ignores: [
      'dist/**',
      'target/**',
      'node_modules/**',
      '.context/**',
      '.agents/**',
      '.codex/**',
      '.cursor/**',
      '.kiro/**',
      'src-tauri/gen/**',
      'src/bindings.ts',
      'vendor/**',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  { languageOptions: { globals: { ...globals.node } } },
  {
    files: ['tests/desktop/**/*.mjs', 'tests/performance/**/*.mjs', 'tests/formats/**/*.mjs'],
    languageOptions: { globals: { ...globals.mocha, ...globals.browser } },
  },
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: { globals: globals.browser },
    plugins: { 'react-hooks': hooks, 'react-refresh': refresh },
    rules: {
      ...hooks.configs.recommended.rules,
      'react-refresh/only-export-components': ['warn', { allowConstantExport: true }],
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: '@chakra-ui/react',
              importNames: [
                'extendTheme',
                'Modal',
                'FormControl',
                'FormLabel',
                'FormErrorMessage',
                'Divider',
                'Collapse',
                'Hide',
                'useToast',
                'useColorMode',
                'useColorModeValue',
              ],
              message: 'Use Chakra v3 compound components and APIs; see AGENTS.md.',
            },
            { name: '@chakra-ui/icons', message: 'Chakra v3 has no icons package.' },
          ],
          patterns: [
            {
              group: ['@/components/*/*'],
              message: 'Import a component through its folder barrel.',
            },
          ],
        },
      ],
      'no-restricted-syntax': [
        'error',
        {
          selector:
            'JSXAttribute[name.name=/^(colorScheme|isOpen|isDisabled|isInvalid|isLoading|isRequired|isReadOnly|isTruncated|noOfLines|sx)$/]',
          message: 'Use the Chakra v3 prop names documented in AGENTS.md.',
        },
        {
          selector:
            'JSXOpeningElement[name.name=/^(Stack|HStack|VStack)$/] > JSXAttribute[name.name="spacing"]',
          message: 'Chakra v3 stacks use gap.',
        },
      ],
    },
  },
);
