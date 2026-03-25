import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import packageJson from './package.json';

export default defineConfig({
  plugins: [vue()],
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
    chunkSizeWarningLimit: 1500,
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
        // Fixed css name so index.ejs can reference it with a <link> tag.
        // (Vite doesn't inject <link> tags when building from a .ts entry point.)
        assetFileNames: (assetInfo) => {
          if (assetInfo.name?.endsWith('.css')) return 'main.css';
          return 'assets/[name]-[hash][extname]';
        },
        preserveModulesRoot: 'src',
        manualChunks: undefined // Disable code-splitting
      },
      preserveEntrySignatures: 'strict',
      treeshake: false,
      checks: {
        pluginTimings: false
      }
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