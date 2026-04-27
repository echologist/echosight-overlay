import {
  getRecord,
  isRecord,
  type ThemeForCss
} from './themeCssHelpers';

export function generateBackgroundStyles(theme: ThemeForCss): string {
  let backgroundCSS = '';
  const mainBgAsset = theme.loadedAssets?.main_bg;
  const backgroundImageAsset = typeof theme.backgroundImage === 'string'
    ? theme.loadedAssets?.[theme.backgroundImage.replace(/\.[^.]+$/, '')]
    : undefined;
  const colors = getRecord(theme.colors);
  const backgroundColors = getRecord(colors?.background);
  const backgroundColor = typeof backgroundColors?.primary === 'string'
    ? backgroundColors.primary
    : undefined;

  if (mainBgAsset || backgroundImageAsset || (backgroundColor && backgroundColor !== 'transparent')) {
    const assetUrl = mainBgAsset || backgroundImageAsset;

    backgroundCSS += `
      body {
        ${backgroundColor && backgroundColor !== 'transparent' ? `background-color: ${backgroundColor} !important;` : ''}
        ${assetUrl ? `
        background-image: url('${assetUrl}') !important;
        background-size: 100% 100% !important;
        background-position: center !important;
        background-repeat: no-repeat !important;
        background-attachment: fixed !important;` : ''}
      }

      .overlay-container.interactive {
        ${assetUrl ? 'background: transparent !important; background-color: transparent !important;' : ''}
      }
      .overlay-container.click-through {
        ${assetUrl ? 'background: transparent !important; background-color: transparent !important;' : ''}
      }
    `;
  }

  if (isRecord(theme.backgrounds)) {
    Object.entries(theme.backgrounds).forEach(([selector, bgConfig]) => {
      backgroundCSS += generateSelectorBackground(selector, bgConfig, theme);
    });
  }

  if (theme.loadedAssets) {
    Object.entries(theme.loadedAssets).forEach(([assetName, assetUrl]) => {
      const assetInfo = theme.assets?.[assetName];
      if (assetInfo?.type === 'background') {
        backgroundCSS += `
          .bg-${assetName} {
            background-image: url('${assetUrl}');
            background-size: cover;
            background-position: center;
            background-repeat: no-repeat;
          }
        `;
      }
    });
  }

  return backgroundCSS + generateAutoAssetStyles(theme);
}

function generateAutoAssetStyles(theme: ThemeForCss): string {
  if (!theme.loadedAssets) return '';

  let autoCSS = '';
  const assetMappings: Record<string, string[]> = {
    titlebar_bg: ['.overlay-container.interactive .header'],
    button_normal: [
      '.overlay-container.interactive button',
      '.overlay-container.interactive .template-btn',
      '.overlay-container.interactive .add-btn',
      '.overlay-container.interactive .header-btn'
    ],
    button_hover: [
      '.overlay-container.interactive button:hover',
      '.overlay-container.interactive .template-btn:hover',
      '.overlay-container.interactive .add-btn:hover'
    ],
    button_pressed: [
      '.overlay-container.interactive button:active',
      '.overlay-container.interactive .template-btn:active',
      '.overlay-container.interactive .add-btn:active'
    ],
    progress_bg: ['.overlay-container.interactive .progress-bar'],
    progress_fill: ['.overlay-container.interactive .progress-fill']
  };

  Object.entries(assetMappings).forEach(([assetName, selectors]) => {
    const assetUrl = theme.loadedAssets?.[assetName];
    if (!assetUrl) return;

    selectors.forEach(selector => {
      autoCSS += `
        ${selector} {
          background-image: url('${assetUrl}') !important;
          background-size: cover !important;
          background-repeat: no-repeat !important;
        }
      `;
    });
  });

  return autoCSS;
}

function generateSelectorBackground(selector: string, bgConfig: unknown, theme: ThemeForCss): string {
  if (!isRecord(bgConfig)) return '';

  let bgCSS = `${selector} {\n`;

  if (typeof bgConfig.asset === 'string' && theme.loadedAssets?.[bgConfig.asset]) {
    bgCSS += `  background-image: url('${theme.loadedAssets[bgConfig.asset]}');\n`;
  }

  if (bgConfig.size) bgCSS += `  background-size: ${bgConfig.size};\n`;
  if (bgConfig.position) bgCSS += `  background-position: ${bgConfig.position};\n`;
  if (bgConfig.repeat) bgCSS += `  background-repeat: ${bgConfig.repeat};\n`;
  if (bgConfig.attachment) bgCSS += `  background-attachment: ${bgConfig.attachment};\n`;
  if (bgConfig.color) bgCSS += `  background-color: ${bgConfig.color};\n`;

  bgCSS += '}\n';
  return bgCSS;
}
