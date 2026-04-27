import {
  getRecord,
  isRecord,
  objectToCSS,
  type ThemeForCss
} from './themeCssHelpers';

export function generateInteractiveStyles(theme: ThemeForCss): string {
  const styles = getRecord(theme.styles);
  const interactive = getRecord(styles?.interactive);
  if (!interactive) return '';

  return `
    .overlay-container.interactive {
      ${objectToCSS(interactive.container)}
    }

    .overlay-container.interactive .header {
      ${objectToCSS(interactive.header)}
    }

    .overlay-container.interactive .template-section {
      ${objectToCSS(interactive.templateSection)}
    }

    .overlay-container.interactive .tasks-section {
      ${objectToCSS(interactive.tasksSection)}
    }
  `;
}

export function generateClickThroughStyles(theme: ThemeForCss): string {
  const styles = getRecord(theme.styles);
  const clickThrough = getRecord(styles?.clickThrough);
  if (!clickThrough) return '';

  return `
    .overlay-container.click-through {
      ${objectToCSS(clickThrough.container)}
    }

    .overlay-container.click-through .tasks-section {
      ${objectToCSS(clickThrough.tasksSection)}
    }
  `;
}

export function generateCommonStyles(theme: ThemeForCss): string {
  const effects = getRecord(theme.effects);

  return `
    ${generateGlobalEffects(theme)}

    .overlay-container .task-text {
      color: var(--text-primary) !important;
      font-weight: var(--font-weight-normal, normal) !important;
      ${effects?.textShadow ? 'text-shadow: var(--text-shadow) !important;' : ''}
    }

    .overlay-container .task-text.completed {
      color: var(--text-muted) !important;
      ${effects?.textShadow ? 'text-shadow: var(--text-shadow) !important;' : ''}
    }

    .overlay-container .progress-text {
      color: var(--text-secondary) !important;
      font-weight: var(--font-weight-bold, bold) !important;
      ${effects?.textShadow ? 'text-shadow: var(--text-shadow) !important;' : ''}
    }

    .overlay-container .progress-bar {
      ${effects?.boxShadow ? 'box-shadow: var(--box-shadow) !important;' : ''}
      ${effects?.borderRadius ? 'border-radius: var(--border-radius) !important;' : ''}
    }

    .overlay-container .task-checkbox {
      ${effects?.dropShadow ? 'filter: var(--drop-shadow) !important;' : ''}
    }

    button, .template-btn, .add-btn, .header-btn, .modal-btn {
      ${effects?.borderRadius ? 'border-radius: var(--border-radius) !important;' : ''}
      ${effects?.buttonShadow ? 'box-shadow: var(--button-shadow) !important;' : ''}
      ${effects?.textShadow ? 'text-shadow: var(--text-shadow) !important;' : ''}
    }

    input, textarea, select, .task-input, .modal-input {
      ${effects?.borderRadius ? 'border-radius: var(--border-radius) !important;' : ''}
      ${effects?.bevelInset ? 'box-shadow: var(--bevel-inset) !important;' : ''}
    }
  `;
}

export function generateCustomCSS(theme: ThemeForCss): string {
  if (!theme.customCSS) return '';

  if (typeof theme.customCSS === 'string') {
    return theme.customCSS;
  }

  if (!isRecord(theme.customCSS)) {
    return '';
  }

  let customCSS = '';
  Object.entries(theme.customCSS).forEach(([key, styles]) => {
    if (typeof styles === 'string') {
      customCSS += `${styles}\n`;
    } else if (isRecord(styles)) {
      customCSS += `
        .theme-${key} {
          ${objectToCSS(styles)}
        }
      `;
    }
  });

  return customCSS;
}

function generateGlobalEffects(theme: ThemeForCss): string {
  if (!isRecord(theme.effects)) return '';

  let effectsCSS = '';

  if (theme.effects.bevelInset || theme.effects.bevelOutset) {
    effectsCSS += `
      .bevel-inset {
        box-shadow: var(--bevel-inset) !important;
      }

      .bevel-outset {
        box-shadow: var(--bevel-outset) !important;
      }
    `;
  }

  if (theme.effects.textShadow) {
    effectsCSS += `
      .text-shadow {
        text-shadow: var(--text-shadow) !important;
      }
    `;
  }

  if (theme.effects.dropShadow) {
    effectsCSS += `
      .drop-shadow {
        filter: var(--drop-shadow) !important;
      }
    `;
  }

  if (theme.effects.glow) {
    effectsCSS += `
      .glow {
        box-shadow: var(--glow) !important;
      }
    `;
  }

  if (theme.effects.blur) {
    effectsCSS += `
      .blur {
        backdrop-filter: var(--blur) !important;
      }
    `;
  }

  return effectsCSS;
}
