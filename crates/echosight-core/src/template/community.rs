use super::model::{TaskTemplate, TemplateTask};
use crate::task::model::{TaskEnvironment, TaskId};

#[derive(Clone, Debug, serde::Serialize)]
pub struct CommunityTemplate {
    pub name: &'static str,
    pub description: &'static str,
    pub tasks: &'static [&'static str],
}

// Verbatim shipped catalog from communityTemplates.ts.
pub const COMMUNITY_TEMPLATES: &[CommunityTemplate] = &[
    CommunityTemplate {
        name: "League Start Essentials",
        description: "Core objectives for starting a new league",
        tasks: &[
            "Reach Act 6 for resistance penalty",
            "Complete Normal Labyrinth",
            "Get life/ES nodes on passive tree",
            "Cap resistances (75%+)",
            "Find/buy movement skill gem",
            "Set up basic currency stash tabs",
            "Get weapon with linked sockets",
            "Reach level 68 for endgame content",
        ],
    },
    CommunityTemplate {
        name: "Endgame Progression",
        description: "Late game goals and pinnacle content",
        tasks: &[
            "Complete Atlas progression",
            "Defeat Shaper",
            "Defeat Elder",
            "Complete all Pinnacle bosses",
            "Reach level 90+",
            "Get 6-link main skill",
            "Accumulate 10+ Divine Orbs",
            "Complete Uber Lab trials",
            "Max out important flasks",
        ],
    },
    CommunityTemplate {
        name: "New Character Setup",
        description: "Essential steps when creating a new character",
        tasks: &[
            "Plan passive tree route (PoB)",
            "Identify skill gem progression",
            "Set up loot filter",
            "Transfer currency from main",
            "Get leveling uniques if available",
            "Join guild/find party",
            "Research build guide thoroughly",
            "Prepare gems for later levels",
        ],
    },
    CommunityTemplate {
        name: "Currency Goals",
        description: "Economic milestones for league",
        tasks: &[
            "Save 1 Divine Orb",
            "Save 5 Divine Orbs",
            "Save 20 Divine Orbs",
            "Save 50 Divine Orbs",
            "Get premium stash tab",
            "Set up efficient farming strategy",
            "Learn market prices for key items",
            "Build up crafting materials",
        ],
    },
    CommunityTemplate {
        name: "HC/SSF Priorities",
        description: "Hardcore and Solo Self-Found specific goals",
        tasks: &[
            "Over-cap resistances (85%+)",
            "Get fortify/defensive layers",
            "Level backup gems",
            "Hoard life flasks",
            "Get movement skills early",
            "Plan escape routes",
            "Avoid risky content until geared",
            "Build defensive passive tree first",
        ],
    },
    CommunityTemplate {
        name: "Crafting Checklist",
        description: "Steps for crafting progression",
        tasks: &[
            "Learn basic crafting recipes",
            "Stockpile crafting orbs",
            "Get good base items",
            "Practice on cheaper items first",
            "Research craft of exile",
            "Set up crafting bench",
            "Learn advanced techniques",
            "Plan expensive crafts carefully",
        ],
    },
];

pub fn create_community_template_tasks(template: &CommunityTemplate) -> Vec<TemplateTask> {
    template
        .tasks
        .iter()
        .map(|text| TemplateTask {
            text: (*text).into(),
            children: Some(vec![]),
            ..Default::default()
        })
        .collect()
}

pub fn create_community_template(
    template: &CommunityTemplate,
    tasks: Vec<TemplateTask>,
    env: &impl TaskEnvironment,
) -> TaskTemplate {
    TaskTemplate {
        id: TaskId(env.now_ms()),
        name: template.name.into(),
        tasks,
        created_at: env.now_iso(),
        imported: false,
        community: true,
    }
}

pub fn create_community_template_preview(template: &CommunityTemplate) -> String {
    let list = template
        .tasks
        .iter()
        .enumerate()
        .map(|(i, text)| format!("{}. {text}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{} Tasks:\n\n{list}", template.name)
}
