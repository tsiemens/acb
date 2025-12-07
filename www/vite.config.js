import { defineConfig } from 'vite';
import packageJson from './package.json';

export default defineConfig({
  build: {
    minify: 'terser',
    terserOptions: {
      compress: {
        // You can still keep other optimizations
        dead_code: false,
        drop_console: false,
        drop_debugger: true
      },
      mangle: false, // This prevents variable/function name mangling
      format: {
        comments: true,
        indent_level: 4,
        semicolons: false // Insert newlines instead of semicolons
      }
    },
    sourcemap: true,
    outDir: 'dist/js',
    assetsDir: 'assets',
    base: '/js/',
    rollupOptions: {
      input: {
        main: './src/main.ts'
      },
      output: {
        entryFileNames: '[name].js',
        chunkFileNames: '[name]-[hash].js',
        preserveModulesRoot: 'src',
        manualChunks: undefined // Disable code-splitting
      },
      preserveEntrySignatures: true,
      treeshake: false
    }
  },
  // Add this to handle WASM files properly
  optimizeDeps: {
    exclude: ['./pkg/acb_wasm.js']
  },

  experimental: {
    // This is a workaround to handle WASM files correctly.
    // Vite generates the URL without the "base" we specify,
    // so they end up thinking /assets is at the root.
    renderBuiltUrl(filename, { hostType }) {
      if (filename.endsWith('.wasm')) {
        return '/js/' + filename;
      }
      // Let Vite handle other assets normally
      return { relative: true };
    }
  },

  resolve: {
    extensions: ['.ts', '.js'],
  },

  define: {
    __PACKAGE_VERSION__: JSON.stringify(packageJson.version),
  }
});