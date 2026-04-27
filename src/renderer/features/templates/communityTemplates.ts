import type { TemplateTask } from '../../../shared/types';

export interface CommunityTemplate {
  name: string;
  description: string;
  tasks: readonly string[];
}

export const COMMUNITY_TEMPLATES = [
  {
    name: 'League Start Essentials',
    description: 'Core objectives for starting a new league',
    tasks: [
      'Reach Act 6 for resistance penalty',
      'Complete Normal Labyrinth',
      'Get life/ES nodes on passive tree',
      'Cap resistances (75%+)',
      'Find/buy movement skill gem',
      'Set up basic currency stash tabs',
      'Get weapon with linked sockets',
      'Reach level 68 for endgame content'
    ]
  },
  {
    name: 'Endgame Progression',
    description: 'Late game goals and pinnacle content',
    tasks: [
      'Complete Atlas progression',
      'Defeat Shaper',
      'Defeat Elder',
      'Complete all Pinnacle bosses',
      'Reach level 90+',
      'Get 6-link main skill',
      'Accumulate 10+ Divine Orbs',
      'Complete Uber Lab trials',
      'Max out important flasks'
    ]
  },
  {
    name: 'New Character Setup',
    description: 'Essential steps when creating a new character',
    tasks: [
      'Plan passive tree route (PoB)',
      'Identify skill gem progression',
      'Set up loot filter',
      'Transfer currency from main',
      'Get leveling uniques if available',
      'Join guild/find party',
      'Research build guide thoroughly',
      'Prepare gems for later levels'
    ]
  },
  {
    name: 'Currency Goals',
    description: 'Economic milestones for league',
    tasks: [
      'Save 1 Divine Orb',
      'Save 5 Divine Orbs',
      'Save 20 Divine Orbs',
      'Save 50 Divine Orbs',
      'Get premium stash tab',
      'Set up efficient farming strategy',
      'Learn market prices for key items',
      'Build up crafting materials'
    ]
  },
  {
    name: 'HC/SSF Priorities',
    description: 'Hardcore and Solo Self-Found specific goals',
    tasks: [
      'Over-cap resistances (85%+)',
      'Get fortify/defensive layers',
      'Level backup gems',
      'Hoard life flasks',
      'Get movement skills early',
      'Plan escape routes',
      'Avoid risky content until geared',
      'Build defensive passive tree first'
    ]
  },
  {
    name: 'Crafting Checklist',
    description: 'Steps for crafting progression',
    tasks: [
      'Learn basic crafting recipes',
      'Stockpile crafting orbs',
      'Get good base items',
      'Practice on cheaper items first',
      'Research craft of exile',
      'Set up crafting bench',
      'Learn advanced techniques',
      'Plan expensive crafts carefully'
    ]
  }
] as const satisfies readonly CommunityTemplate[];

export function createCommunityTemplateTasks(template: CommunityTemplate): TemplateTask[] {
  return template.tasks.map(text => ({
    text,
    children: []
  }));
}
