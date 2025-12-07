/**
 * Dynamically injected npm package version
 */
declare const __PACKAGE_VERSION__: string;

/**
 * Export the npm package version as a constant.
 * This version isn't really updated, as the package is not published.
 */
export const packageVersion = __PACKAGE_VERSION__;

/**
 * This version is does not really comply with npm's recommended semver format,
 * but aligns more with the ACB versioning scheme (though it is not necessarily the
 * same).
 *
 * The main ACB app version is fetched via API call to get_acb_version().
 */
export const webappVersion = "0.25.12";
