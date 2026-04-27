import {
  isRecord,
  objectToCSS,
  toKebabCase,
  type ThemeForCss
} from './themeCssHelpers';
import {
  animationPropertyMap,
  animationSelectorMap
} from './themeCssSelectors';

export function generateAnimationStyles(theme: ThemeForCss): string {
  if (!isRecord(theme.animations)) return '';

  let animationCSS = '';
  const animations = theme.animations;

  if (isRecord(animations.keyframes)) {
    Object.entries(animations.keyframes).forEach(([name, keyframe]) => {
      animationCSS += `
        @keyframes ${name} {
          ${generateKeyframeCSS(keyframe)}
        }
      `;
    });
  }

  if (isRecord(animations.components)) {
    Object.entries(animations.components).forEach(([component, animConfig]) => {
      const selectors = animationSelectorMap[component] || [];
      if (selectors.length > 0) {
        animationCSS += `
          ${selectors.join(', ')} {
            ${generateAnimationProperties(animConfig)}
          }
        `;
      }
    });
  }

  if (isRecord(animations.selectors)) {
    Object.entries(animations.selectors).forEach(([selector, animConfig]) => {
      animationCSS += `
        ${selector} {
          ${generateAnimationProperties(animConfig)}
        }
      `;
    });
  }

  if (animations.transitions) {
    animationCSS += generateTransitionStyles(animations.transitions);
  }

  return animationCSS;
}

function generateKeyframeCSS(keyframe: unknown): string {
  if (!isRecord(keyframe)) return '';

  return Object.entries(keyframe).map(([percentage, styles]) => `
    ${percentage} {
      ${objectToCSS(styles)}
    }
  `).join('');
}

function generateAnimationProperties(animConfig: unknown): string {
  if (!isRecord(animConfig)) return '';

  const properties: string[] = [];
  const cssKeys = [
    'name', 'duration', 'timing', 'delay', 'iteration', 'direction', 'fillMode', 'playState',
    'animation',
    'transform', 'transformOrigin',
    'transition', 'transitionProperty', 'transitionDuration', 'transitionTiming', 'transitionDelay'
  ];

  cssKeys.forEach(key => {
    if (!animConfig[key]) return;

    const cssKey = animationPropertyMap[key] || toKebabCase(key);
    properties.push(`${cssKey}: ${animConfig[key]} !important`);
  });

  return properties.join(';\n      ');
}

function generateTransitionStyles(transitionConfig: unknown): string {
  if (!isRecord(transitionConfig)) return '';

  let transitionCSS = '';

  if (transitionConfig.global) {
    transitionCSS += `
      * {
        transition: ${transitionConfig.global} !important;
      }
    `;
  }

  if (isRecord(transitionConfig.components)) {
    Object.entries(transitionConfig.components).forEach(([component, transition]) => {
      const selectors = animationSelectorMap[component] || [];
      if (selectors.length > 0) {
        transitionCSS += `
          ${selectors.join(', ')} {
            transition: ${transition} !important;
          }
        `;
      }
    });
  }

  if (isRecord(transitionConfig.hover)) {
    Object.entries(transitionConfig.hover).forEach(([selector, hoverConfig]) => {
      transitionCSS += `
        ${selector}:hover {
          ${generateAnimationProperties(hoverConfig)}
        }
      `;
    });
  }

  return transitionCSS;
}
