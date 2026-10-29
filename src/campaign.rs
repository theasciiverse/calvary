//! The campaign (CAMPAIGN.md): missions in order, each a `.cmap` built into
//! the binary with its own win and lose rules, a briefing over the globe
//! turned to its place, and the original's own result screens after it
//! (`youwin.jpg` and `youlose.jpg` from the 1998 release). The furthest
//! mission reached is saved, so the title can continue from it.

use crate::audio::Audio;
use crate::input::Input;
use crate::title;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::io;
use std::time::{Duration, Instant};

const AMBER: Color = Color::Rgb(255, 180, 40);
const GREEN: Color = Color::Rgb(80, 220, 120);
const DIM: Color = Color::Rgb(70, 110, 80);
const BONE: Color = Color::Rgb(226, 220, 204);

pub struct Act {
    pub name: &'static str,
    /// Kruese's press conference, the "Golgotha Transmission", opens each act.
    pub transmission: &'static [&'static str],
}

pub struct Mission {
    /// Index into `ACTS`.
    pub act: usize,
    pub number: u32,
    pub title: &'static str,
    /// The map, built in (`maps/*.cmap`).
    pub map: &'static str,
    /// Sergeant Reid Charles on the radio.
    pub briefing: &'static [&'static str],
    pub objective: &'static str,
}

pub const ACTS: &[Act] = &[
    Act {
        name: "Act I: The Incursion",
        transmission: &["We went in because they told us an American was dead.", "She was. They lied about who killed her."],
    },
    Act { name: "Act II: The Desertion", transmission: &["After Nineveh we had no flag.", "We had a notebook and a sergeant who could read it."] },
    Act { name: "Act III: The European War", transmission: &["Europe was not fighting America.", "Europe was fighting on schedule."] },
    Act { name: "Act IV: The North", transmission: &["Svalbard is where we stopped calling them sites.", "They were never graves. They were beds."] },
];

pub const MISSIONS: &[Mission] = &[
    Mission {
        act: 0,
        number: 1,
        title: "Al Basrah",
        map: include_str!("../maps/al-basrah.cmap"),
        briefing: &[
            "Commander, Reid here. We are ashore on the Shatt al-Arab, south of the",
            "old port, and the HQ is down. Command says the people who killed",
            "Dr. Calloway came through Basrah, and the road north is held: towers",
            "along the waterfront, armour in the date groves, more of it coming.",
            "No garage until the engineers land, sir. It is you and the supertank.",
            "When the guns run dry, come home: the HQ rearms you and patches you up.",
        ],
        objective: "Destroy every enemy tank and tower. Keep the HQ standing.",
    },
    Mission {
        act: 0,
        number: 2,
        title: "Shatt al-Arab",
        map: include_str!("../maps/shatt-al-arab.cmap"),
        briefing: &[
            "Commander, Reid. The engineers are ashore and we have a garage at last.",
            "North of here the marsh is cut by three channels, and every bridge on",
            "both causeways is blown. Peon tanks will hold the line, but nothing",
            "crosses water without a bridger: send one up a causeway and it lays",
            "itself over the first gap it finds. Their base sits on the levee to",
            "the north-east. Shell the far banks before you send the bridgers in.",
        ],
        objective: "Bridge the channels and destroy the enemy HQ. Keep ours standing.",
    },
    Mission {
        act: 0,
        number: 3,
        title: "Ur",
        map: include_str!("../maps/ur.cmap"),
        briefing: &[
            "Commander, this is Ur. Dr. Calloway's first dig is at the foot of the",
            "ziggurat, and Command wants it in our hands before anyone else goes",
            "through her trenches. Hold the dig for ninety seconds with nothing of",
            "theirs on it. There are banks along both roads: an engineer takes one,",
            "and our money planes fly its crates home. Their engineers know that too.",
            "Guns on the ziggurat's first terrace cover the dig. Mind them, sir.",
        ],
        objective: "Hold Calloway's dig for 90 seconds. Keep the HQ standing.",
    },
    Mission {
        act: 0,
        number: 4,
        title: "Nakhayb",
        map: include_str!("../maps/nakhayb.cmap"),
        briefing: &[
            "Commander, Reid. Nakhayb, out in the western desert. This was her camp:",
            "Calloway worked here alone for two seasons, and she left in a hurry.",
            "It sits where the two wadis meet. Get an engineer onto the pad in the",
            "middle and the camp is ours, with a turret to keep it. Theirs will try",
            "the same: if they get there first, knock their turret down and go again.",
            "The mesas are sheer, sir. Stay in the wadis.",
        ],
        objective: "Take Calloway's camp: an engineer on the takeover pad. Keep the HQ standing.",
    },
    Mission {
        act: 0,
        number: 5,
        title: "Babylon",
        map: include_str!("../maps/babylon.cmap"),
        briefing: &[
            "Commander. Her notebook names a second site, east of the old city of",
            "Babylon, and they got there first. Sir, I have seen it from the ridge.",
            "It is bones. A skull the size of a truck, ribs taller than our tanks,",
            "lying in a pit in the open. They have ringed it with electric towers.",
            "Bring those towers down, or send engineers to take them. Our garage",
            "can build electric cars now: short reach, but they burn through armour.",
        ],
        objective: "Destroy or take every electric tower round the pit. Keep the HQ standing.",
    },
    Mission {
        act: 0,
        number: 6,
        title: "The Euphrates line",
        map: include_str!("../maps/euphrates.cmap"),
        briefing: &[
            "Commander, Reid. Six trucks for the forward post on the Euphrates,",
            "and the bones from Babylon are in two of them. Command has changed",
            "the route twice since dusk, so choose for yourself, sir: J switches",
            "the junction nearest you, and the crossovers let you change your mind.",
            "Both roads are watched. Select the trucks and X holds them while you",
            "clear the way; R sends them on. Four must get through. Lose three and",
            "it is over.",
        ],
        objective: "Bring four of the six supply trucks to the forward post. Keep the HQ standing.",
    },
    Mission {
        act: 0,
        number: 7,
        title: "Nineveh",
        map: include_str!("../maps/nineveh.cmap"),
        briefing: &[
            "Sir. The order came in an hour ago: raze the site at Nineveh, and",
            "burn the bones. You said no on an open channel. The company at the",
            "south gate was ours this morning; it is not ours now. Blue on the map",
            "is American, red is European, and they will fight each other as soon",
            "as us. Hold the site until the column is ready to turn west. Two",
            "minutes with nothing of theirs on it, sir. Then we are deserters.",
        ],
        objective: "Hold the site for two minutes against both armies. Keep the HQ standing.",
    },
    Mission {
        act: 1,
        number: 8,
        title: "Palmyra",
        map: include_str!("../maps/palmyra.cmap"),
        briefing: &[
            "Commander, Reid. The column is through Palmyra and strung out on the",
            "road west, trucks and wounded first. The Americans are an hour behind",
            "us and closing: three roads into the old city, and every one of them",
            "ends at our camp by the colonnade. Five minutes, sir. Hold them in the",
            "ruins for five minutes and the column is clear. Her notebook says the",
            "Temple of Bel was built on something older. We have no time to look.",
        ],
        objective: "Hold out for five minutes. Keep the HQ standing.",
    },
    Mission {
        act: 1,
        number: 9,
        title: "The King's Highway",
        map: include_str!("../maps/kings-highway.cmap"),
        briefing: &[
            "Commander. Six trucks south, the whole column's stores. The King's",
            "Highway runs past Kerak, and the Europeans hold the castle; the desert",
            "road east is longer, and the Americans are already on it. Both cross",
            "Wadi Mujib. The road west goes to Jerusalem, sir. It is blocked, and",
            "Calloway's notebook says the same thing every time it names the city:",
            "not yet. Four trucks must reach the post at the south end.",
        ],
        objective: "Bring four of the six trucks to the south post. Keep the HQ standing.",
    },
    Mission {
        act: 1,
        number: 10,
        title: "Sinai",
        map: include_str!("../maps/sinai.cmap"),
        briefing: &[
            "Commander, Reid. We crossed the Gulf with what we could carry, and it",
            "is not much: four thousand in the chest and no bank of our own. There",
            "are three banks on the way west. Take them with engineers and the",
            "money plane flies their crates home; that is all the supply we have.",
            "The Europeans hold the Mitla Pass, guns on both ridges, and they have",
            "rocket tanks out in the dunes. Their camp is on the far side.",
        ],
        objective: "Destroy the European HQ beyond the Mitla Pass. Keep ours standing.",
    },
    Mission {
        act: 1,
        number: 11,
        title: "The Suez Canal",
        map: include_str!("../maps/suez.cmap"),
        briefing: &[
            "Sir. Every bridge on the canal is down; at three places the old piers",
            "still reach out from both banks, a gap of water between them. A bridger",
            "closes a gap, and the trucks are waiting to cross. The Europeans hold",
            "the west bank and will shoot the decks out from under us, so keep",
            "bridgers coming. The Americans are coming down the east bank behind.",
            "Four trucks across to the post on the far side, Commander.",
        ],
        objective: "Bridge the canal and bring four of the six trucks to the west post. Keep the HQ standing.",
    },
    Mission {
        act: 1,
        number: 12,
        title: "Cairo",
        map: include_str!("../maps/cairo.cmap"),
        briefing: &[
            "Commander, Reid. Cairo, the old walled town on the terraces. We hold the",
            "north-east corner and an airfield; the Europeans hold the south-east,",
            "and between us it is street by street, the ring road or the long way",
            "round by the west gate, where two of our launchers still stand. The",
            "airbase builds helicopters now, sir: they fly the roads like the tanks,",
            "only over the walls. Their HQ, and we have Cairo.",
        ],
        objective: "Destroy the European HQ in the south-east. Keep ours standing.",
    },
    Mission {
        act: 1,
        number: 13,
        title: "Giza",
        map: include_str!("../maps/giza.cmap"),
        briefing: &[
            "Sir. Her notebook puts the next site under the Giza plateau, and the",
            "Europeans are already digging at the foot of the great pyramid. But",
            "that is not all of it. Our scouts saw tanks come up out of the Valley",
            "Temple below the Sphinx: black hulls, a white band, no flag. They shot",
            "at the Europeans, and then at us. Whoever they are, they came from",
            "under the plateau. Find where they come out, and shut it.",
        ],
        objective: "Destroy the black-banded army's garage in the Valley Temple. Keep the HQ standing.",
    },
    Mission {
        act: 1,
        number: 14,
        title: "Alexandria",
        map: include_str!("../maps/alexandria.cmap"),
        briefing: &[
            "Commander, Reid. Three ships in the Eastern Harbour, and two will carry",
            "the column to Greece. The Europeans hold the waterfront. Each ship is",
            "boarded from a pad on the quay: an engineer on the pad, and she is ours.",
            "And sir, their jets are already up. Bombers with them, coming over the",
            "city for our camp. Get under cover, then get to the quay.",
            "Two ships, and we sail.",
        ],
        objective: "Take two of the three ships with engineers on their quay pads. Keep the HQ standing.",
    },
    Mission {
        act: 2,
        number: 15,
        title: "Piraeus",
        map: include_str!("../maps/piraeus.cmap"),
        briefing: &[
            "Commander, Reid. Piraeus. The ships put us ashore west of the river",
            "mouth, and the Europeans have a whole base across it: garage, airbase,",
            "banks up in the hills by the lake. Calloway's notebook lists Athens",
            "next, and every one of her sites so far has had a battle on top of it.",
            "This is a real one, sir: both sides building, both sides with planes.",
            "Take their HQ and the road inland is ours.",
        ],
        objective: "Destroy the European HQ across the river. Keep ours standing.",
    },
    Mission {
        act: 2,
        number: 16,
        title: "Delphi",
        map: include_str!("../maps/delphi.cmap"),
        briefing: &[
            "Commander, Reid. Delphi. The sanctuary is on the slope of Parnassus, and",
            "the Europeans have put superguns on the heights above it: one at the",
            "stadium under the cliffs, one at the Marmaria, one on the western spur.",
            "Each one lobs a shell that takes out anything within three cells, one",
            "every twenty seconds. You can see the shell coming, sir. Keep moving.",
            "Knock them out, or better, get an engineer onto one and turn it round.",
        ],
        objective: "Take or destroy all three European superguns. Keep the HQ standing.",
    },
    Mission {
        act: 2,
        number: 17,
        title: "The Ionian crossing",
        map: include_str!("../maps/ionian.cmap"),
        briefing: &[
            "Commander, Reid. The boats for Italy are waiting, but nothing crosses",
            "while the Europeans own these islands. Lefkada first: there is a",
            "causeway, and a supergun at the end of it watching the strait. Their HQ",
            "and airbase are out on Kefalonia, past Ithaca, where no tank can follow.",
            "The airbase can build bombers now, sir. Send them over the water, and",
            "keep some guns at home: their helicopters will come the other way.",
        ],
        objective: "Destroy the airbase on Kefalonia and the supergun on Lefkada. Keep the HQ standing.",
    },
    Mission {
        act: 2,
        number: 18,
        title: "Naples",
        map: include_str!("../maps/naples.cmap"),
        briefing: &[
            "Commander, Reid. Italy. The Europeans hold Naples, and they are digging",
            "at Herculaneum, on the shore under Vesuvius. Calloway's notebook has it",
            "underlined twice. And sir, the mountain woke up the day they broke",
            "ground. Lava is coming down three channels, one across the coast road.",
            "It burns anything that drives into it, and the crater is throwing rock",
            "onto the slopes. Go round, or go fast. Then hold the dig.",
        ],
        objective: "Hold the dig at Herculaneum for two minutes. Keep the HQ standing.",
    },
    Mission {
        act: 2,
        number: 19,
        title: "Pompeii",
        map: include_str!("../maps/pompeii.cmap"),
        briefing: &[
            "Commander, Reid. Pompeii, and it is dark, and the ash is still coming",
            "down. In 2047 Calloway found a bronze casket under the Villa of the",
            "Mysteries, outside the Herculaneum Gate, and had it walled back in. The",
            "Europeans have her report. They are camped south-west of the town and",
            "they will go for it tonight. Whoever reaches it carries it; whoever",
            "kills the carrier can pick it up. We see nothing past our own lights.",
        ],
        objective: "Bring the casket from the Villa of the Mysteries to our HQ. Do not let the Europeans take it to theirs.",
    },
    Mission {
        act: 2,
        number: 20,
        title: "Rome",
        map: include_str!("../maps/rome.cmap"),
        briefing: &[
            "Commander, Reid. Rome. We are outside the walls on the Via Appia, and the",
            "Europeans hold the hills: their HQ on the Palatine, a factory on the",
            "Esquiline, guns on every hilltop and a supergun on the Quirinal. We will",
            "not starve them out, sir. The engineers found us something better:",
            "tank busters, rockets that hit like a building falling on you, and",
            "cobras that burn. Up the hills, and hold the Palatine.",
        ],
        objective: "Hold the top of the Palatine for two minutes. Keep the HQ standing.",
    },
    Mission {
        act: 2,
        number: 21,
        title: "Turin",
        map: include_str!("../maps/turin.cmap"),
        briefing: &[
            "Commander, Reid. Turin. Sir, they have the Shroud. Not the Europeans:",
            "the Order, in its own colours, black and bone. They took it from the",
            "cathedral an hour ago and it is on a truck going out through the city",
            "to the Alpine road, with an escort. If it gets out we never see it",
            "again. Stop the truck, pick the Shroud up, and bring it here. The",
            "Europeans in the Lingotto factories will want it too.",
        ],
        objective: "Take the Shroud from the Order's truck and bring it to our HQ, before it leaves by the Alpine road.",
    },
    Mission {
        act: 2,
        number: 22,
        title: "The Gotthard Pass",
        map: include_str!("../maps/gotthard.cmap"),
        briefing: &[
            "Commander, Reid. The Shroud is safe and going north with us, over the",
            "Gotthard: six trucks, up the Tremola from Airolo, over the pass, down",
            "through Andermatt, where the Europeans are dug in, and out through the",
            "gorge. Sir, the snow up there is hanging over the road in cornices.",
            "One shell near one and the whole slope comes down on whatever is below.",
            "Theirs or ours. Use that, and watch where they are shooting.",
        ],
        objective: "Get four of the six supply trucks out through the Schollenen gorge. Lose three and it fails.",
    },
    Mission {
        act: 2,
        number: 23,
        title: "Zurich",
        map: include_str!("../maps/zurich.cmap"),
        briefing: &[
            "Commander, Reid. Zurich, and we are broke, sir. The column has run on",
            "what we took since Basrah, and the north will cost more than that. The",
            "valley is full of banks, and nobody is minding them: five of them along",
            "the roads round the lake. Get engineers into them before the Europeans",
            "in the south of the valley do. Four banks of our own and we can pay",
            "for whatever comes after. And sir: a bank they take, we can shoot.",
        ],
        objective: "Own four banks: ours and three more, taken by engineers. Keep the HQ standing.",
    },
    Mission {
        act: 2,
        number: 24,
        title: "Chartres",
        map: include_str!("../maps/chartres.cmap"),
        briefing: &[
            "Commander, Reid. Chartres. The cathedral keeps the Veil, and the Order",
            "wants it as it wanted the Shroud. This time we got here first, sir: we",
            "hold the town on the hill. They are coming from the east, and the",
            "Europeans from over the Eure, and I do not think either knows the other",
            "is coming. Six minutes and the convoy from Paris reaches us. Keep the",
            "Veil in this town until then.",
        ],
        objective: "Hold out for six minutes. Neither the Order nor the Europeans may carry the Veil home.",
    },
    Mission {
        act: 2,
        number: 25,
        title: "The Loire",
        map: include_str!("../maps/loire.cmap"),
        briefing: &[
            "Commander, Reid. We found where they come from. The Order has a chateau",
            "on the north bank of the Loire: a curtain wall, a moat, two gates, and",
            "their HQ in the courtyard. There is a supergun in there too, and it",
            "reaches further than our busters do, so it goes first. Two bridges over",
            "the river, sir. Nobody has ever attacked these people where they live.",
            "Let us be the first.",
        ],
        objective: "Destroy the Order's HQ inside the chateau walls. Keep ours standing.",
    },
    Mission {
        act: 2,
        number: 26,
        title: "Munich",
        map: include_str!("../maps/munich.cmap"),
        briefing: &[
            "Commander, Reid. The papers in the chateau led here. The Order builds",
            "its own armour, sir, in Munich: two factories at the Allach works and",
            "one at the Moosach rail yard, north-west of the old town. There is a",
            "line of new cobras in the works yard, and they will come for us the",
            "moment they know we are here. Over the Isar, and shut the factories",
            "down, all three.",
        ],
        objective: "Destroy the Order's three factories at Allach and Moosach. Keep the HQ standing.",
    },
    Mission {
        act: 2,
        number: 27,
        title: "Vienna",
        map: include_str!("../maps/vienna.cmap"),
        briefing: &[
            "Commander, Reid. Vienna. The Holy Lance is in the treasury of the",
            "Hofburg, in the old city inside the Ring, and this time everyone knows",
            "it. The Order is coming in from the west, the Europeans hold the north",
            "over the canal and have guns on the Ring. We are in the Prater. It is a",
            "race, sir: whoever gets the Lance out of the old city and home keeps it.",
            "After this one, Calloway's notebook goes north.",
        ],
        objective: "Bring the Holy Lance from the Hofburg treasury to our HQ. Nobody else may carry it home.",
    },
    Mission {
        act: 3,
        number: 28,
        title: "Berlin",
        map: include_str!("../maps/berlin.cmap"),
        briefing: &[
            "Commander, Reid. Berlin. Calloway's notebook ends with an address in the",
            "Tiergarten, and it is the Order's command. The Europeans say it is empty,",
            "sir, and for once I believe them: a garrison, a factory and some guns,",
            "nothing like Munich. Whatever they kept in there, they left it. Get an",
            "engineer inside before the Europeans come down from the Spree and burn",
            "it.",
        ],
        objective: "Take the Order's command in the Tiergarten with an engineer. Keep the HQ standing.",
    },
    Mission {
        act: 3,
        number: 29,
        title: "The Baltic crossing",
        map: include_str!("../maps/baltic.cmap"),
        briefing: &[
            "Commander, Reid. The map in the Order's command has forty points on it,",
            "and a line from Berlin north into Finland. The Finns will meet us if we",
            "can get to them. Six trucks over the frozen gulf, sir. The ice holds a",
            "tank, but not a supergun shell, and the Europeans have two of them on",
            "the islands and battleships in the lead. A road with a hole in it is",
            "closed; take another. Our planes fly off the carrier in the channel.",
        ],
        objective: "Bring four of the six trucks over the ice to the Finnish shore. Lose three and it is over.",
    },
    Mission {
        act: 3,
        number: 30,
        title: "Helsinki",
        map: include_str!("../maps/helsinki.cmap"),
        briefing: &[
            "Commander, Reid. The Finns kept their word, sir. Their army holds the",
            "Senate Square and they are fighting beside us from today: white tanks,",
            "blue stripe, do not shoot them. The Europeans hold Katajanokka over the",
            "North Harbour, an airfield in Kallio, a battleship in the South Harbour",
            "and a supergun out on Suomenlinna. The harbour is frozen; there is a",
            "road over the ice. Their HQ goes, and nobody loses theirs.",
        ],
        objective: "Destroy the Europeans' HQ on Katajanokka. Neither our HQ nor the Finns' may fall.",
    },
    Mission {
        act: 3,
        number: 31,
        title: "The Finnish lakes",
        map: include_str!("../maps/lakes.cmap"),
        briefing: &[
            "Commander, Reid. Lake country, sir. Two chains of lakes from one side",
            "of the map to the other, and the Europeans blew every bridge on the",
            "three roads north as they went: six gaps, one cell of water each. The",
            "bridgers are back on the list. A Finnish detachment is with us in the",
            "south-east. Their HQ is past the second chain, and they will be",
            "shooting at whatever we lay.",
        ],
        objective: "Bridge the narrows and destroy the Europeans' HQ in the north. Keep ours standing.",
    },
    Mission {
        act: 3,
        number: 32,
        title: "Lapland",
        map: include_str!("../maps/lapland.cmap"),
        briefing: &[
            "Commander, Reid. Lapland, and the sun does not come up here this time",
            "of year. The Order has two camps somewhere north of us in the fells,",
            "each with a factory, and their convoys come down over the frozen",
            "lakes. Three roads out of camp and nobody can tell me where they go.",
            "We will see what our tanks see and nothing else, sir. Mind the",
            "reindeer.",
        ],
        objective: "Find and destroy the Order's two factories in the fells. Keep the HQ standing.",
    },
    Mission {
        act: 3,
        number: 33,
        title: "The fjord",
        map: include_str!("../maps/fjord.cmap"),
        briefing: &[
            "Commander, Reid. Norway. One road along the north side of the fjord,",
            "a ledge with the mountain straight up on one side and straight down on",
            "the other. Three streams come down across it and the Order blew all",
            "three bridges. Two of their battleships lie under the ledge. At the",
            "head of the fjord they have a supertank of their own, sir, the first",
            "we have seen. Ours is better. Show them.",
        ],
        objective: "Bridge the streams, beat the Order's supertank and destroy their HQ at the fjord head.",
    },
    Mission {
        act: 3,
        number: 34,
        title: "Svalbard",
        map: include_str!("../maps/svalbard.cmap"),
        briefing: &[
            "Commander, Reid. Svalbard. Calloway was here in the last month before",
            "they killed her, sir, and she left her letters in the vault cut into",
            "the mountain over the fjord. Everybody knows it now: the Order is on",
            "the north side, the Europeans east of us on ours. Our road goes over",
            "the fjord ice; so does theirs, and ice breaks. I want to read what she",
            "wrote before anyone else does.",
        ],
        objective: "Bring Calloway's letters from the vault to our HQ. Nobody else may carry them home.",
    },
    Mission {
        act: 3,
        number: 35,
        title: "The Nephilim Battle",
        map: include_str!("../maps/nephilim.cmap"),
        briefing: &[
            "Commander, Reid. Jotunheimen. The name means the home of the giants,",
            "and that is not a figure of speech any more, sir. They are under the",
            "ice of the lake, a dozen of them, and the Order is on the north shore",
            "driving its tanks over them to wake them. Anything that drives near",
            "one wakes it. The thaw is seven minutes away, the letters say, and",
            "then the ice takes them back. Hold the south shore until it does.",
        ],
        objective: "Hold the south shore for seven minutes, until the ice breaks. Keep the HQ standing.",
    },
];

/// The mission's map, parsed.
pub fn map(m: &Mission) -> Result<(crate::level::Level, String, Option<String>), String> {
    crate::cmap::parse(m.map).map_err(|e| format!("mission {} ({}): {e}", m.number, m.title))
}

/// Where the mission is, for the globe.
pub fn place(m: &Mission) -> (&'static str, f32, f32) {
    title::place_of(m.title).unwrap_or(title::GOLGOTHA)
}

fn progress_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("", "", "calvary").map(|d| d.data_dir().join("campaign.txt"))
}

/// How many missions are open to play: the first, plus every one after a
/// mission won.
pub fn reached() -> usize {
    let n = progress_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("reached = ").and_then(|v| v.trim().parse::<usize>().ok())))
        .unwrap_or(1);
    n.clamp(1, MISSIONS.len())
}

/// Open the mission after `index` (a no-op when it already is).
pub fn unlock_after(index: usize) {
    let n = (index + 2).min(MISSIONS.len());
    if n <= reached() {
        return;
    }
    let Some(path) = progress_path() else { return };
    let saved = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&path, format!("reached = {n}\n")));
    match saved {
        Ok(()) => log::info!("campaign: {n} missions open ({})", path.display()),
        Err(e) => log::warn!("campaign progress not saved to {}: {e}", path.display()),
    }
}

/// What the player chose on a campaign screen.
#[derive(PartialEq, Debug)]
pub enum Pick {
    Go,
    Back,
}

/// One key, click or pad button: `Some(true)` goes on, `Some(false)` backs out.
fn read_choice(input: &mut Input, wait: Duration) -> io::Result<Option<bool>> {
    input.poll_pad();
    input.unit_key = None;
    input.zoom_step = 0;
    input.menu_moves = 0;
    if input.build.take().is_some() || std::mem::take(&mut input.toggle_help) {
        return Ok(Some(true));
    }
    if std::mem::take(&mut input.switch_route) {
        return Ok(Some(false));
    }
    if !event::poll(wait)? {
        return Ok(None);
    }
    Ok(match event::read()? {
        Event::Key(k) if k.kind == KeyEventKind::Press => match k.code {
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                input.exit_app = true;
                Some(false)
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace => Some(false),
            _ => Some(true),
        },
        Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => Some(true),
        _ => None,
    })
}

/// Kruese's words in quotation marks, opening on the first line.
fn quoted(lines: &[&str]) -> Vec<String> {
    let n = lines.len();
    lines.iter().enumerate().map(|(i, l)| format!("{}{l}{}", if i == 0 { "\u{201c}" } else { "" }, if i + 1 == n { "\u{201d}" } else { "" })).collect()
}

/// Lines typed out left-aligned in a block centred on the screen, so a line
/// being typed stays put; each line on its own black backing.
fn block(f: &mut Frame, area: Rect, y: u16, full: &[String], typed: &[String], style: Style) -> u16 {
    let w = full.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    for (i, l) in typed.iter().enumerate() {
        let row = y + i as u16;
        if row < area.y + area.height && !l.is_empty() {
            let r = Rect { x, y: row, width: (l.chars().count() as u16).min(area.width), height: 1 };
            f.render_widget(Paragraph::new(Span::styled(l.clone(), style.bg(Color::Black))), r);
        }
    }
    full.len() as u16
}

/// Text over the globe, each line on its own black backing.
fn centred(f: &mut Frame, area: Rect, y: u16, spans: Vec<Span>) {
    if y >= area.y + area.height {
        return;
    }
    let spans: Vec<Span> = spans.into_iter().map(|sp| {
            let style = sp.style;
            if style.bg.is_none() { sp.style(style.bg(Color::Black)) } else { sp }
        }).collect();
    f.render_widget(Paragraph::new(Line::from(spans)).centered(), Rect { y, height: 1, ..area });
}

/// The briefing: the globe turns to the mission's place while the act's
/// transmission (on its first mission) and Reid's radio call type out.
pub fn briefing(terminal: &mut DefaultTerminal, audio: &mut Audio, input: &mut Input, index: usize) -> io::Result<Pick> {
    let m = &MISSIONS[index];
    let act = &ACTS[m.act];
    let opens_act = MISSIONS.iter().position(|x| x.act == m.act) == Some(index);
    if let Ok((_, _, Some(music))) = map(m) {
        audio.play_music(&music);
    }
    let (place, lat, lon) = place(m);
    // the globe swings in from Golgotha
    let mut view = (title::GOLGOTHA.2.to_radians(), (title::GOLGOTHA.1 * 0.8).to_radians());
    let start = Instant::now();
    let mut last = Instant::now();
    let mut typed_all = false;
    loop {
        let t = start.elapsed().as_secs_f32();
        let k = (last.elapsed().as_secs_f32() * 1.5).min(1.0);
        last = Instant::now();
        let d = (lon.to_radians() - view.0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        view.0 += d * k;
        view.1 += ((lat * 0.8).to_radians() - view.1) * k;
        // characters typed so far: the transmission, then the radio call
        let typed = if typed_all { usize::MAX } else { (t * 55.0) as usize };
        let total: usize = quoted(act.transmission).iter().map(|l| l.chars().count()).sum::<usize>() + m.briefing.iter().map(|l| l.chars().count()).sum::<usize>();
        terminal.draw(|f| draw_briefing(f, m, act, opens_act, (place, lat, lon), view, t, typed))?;
        match read_choice(input, Duration::from_millis(33))? {
            Some(true) if !typed_all && typed < total => typed_all = true,
            Some(true) => return Ok(Pick::Go),
            Some(false) => return Ok(Pick::Back),
            None => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_briefing(f: &mut Frame, m: &Mission, act: &Act, opens_act: bool, place: (&str, f32, f32), view: (f32, f32), t: f32, typed: usize) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(Color::Black)), area);
    title::draw_globe(f.buffer_mut(), area, area.height, t, 0.4, view, Some((place.1, place.2)));
    let mut left = typed;
    let mut type_out = |text: &str| -> String {
        let n = text.chars().count().min(left);
        left -= n;
        text.chars().take(n).collect()
    };
    let words = if opens_act { quoted(act.transmission) } else { Vec::new() };
    let transmission: Vec<String> = words.iter().map(|l| type_out(l)).collect();
    let call: Vec<String> = m.briefing.iter().map(|l| l.to_string()).collect();
    let radio: Vec<String> = call.iter().map(|l| type_out(l)).collect();
    let done = left > 0;
    // laid out from a top that leaves the block centred low on the screen
    let rows = 3 + if opens_act { act.transmission.len() as u16 + 3 } else { 0 } + 2 + radio.len() as u16 + 4;
    let mut y = area.y + area.height.saturating_sub(rows + 1).max(1);
    let bold = |c: Color| Style::default().fg(c).add_modifier(Modifier::BOLD);
    centred(f, area, y, vec![Span::styled(act.name.to_uppercase(), bold(AMBER))]);
    y += 2;
    if opens_act {
        y += block(f, area, y, &words, &transmission, Style::default().fg(BONE).add_modifier(Modifier::ITALIC));
        centred(f, area, y, vec![Span::styled("Lt. Elaine Kruese, the Golgotha Transmission", Style::default().fg(DIM))]);
        y += 2;
    }
    let ns = if place.1 >= 0.0 { 'N' } else { 'S' };
    let ew = if place.2 >= 0.0 { 'E' } else { 'W' };
    centred(
        f,
        area,
        y,
        vec![
            Span::styled(format!("MISSION {}: {}", m.number, m.title.to_uppercase()), bold(GREEN)),
            Span::styled(format!("   {:.1}°{ns} {:.1}°{ew}", place.1.abs(), place.2.abs()), Style::default().fg(AMBER)),
        ],
    );
    y += 2;
    y += block(f, area, y, &call, &radio, Style::default().fg(GREEN));
    centred(f, area, y, vec![Span::styled("Sgt. Reid Charles, on the radio", Style::default().fg(DIM))]);
    y += 2;
    if done {
        centred(f, area, y, vec![Span::styled("Objective: ", bold(AMBER)), Span::styled(m.objective, Style::default().fg(BONE))]);
        centred(f, area, y + 2, vec![Span::styled("Enter: move out    F1 in battle: controls    Esc: back to the title", Style::default().fg(DIM))]);
    }
}

static YOU_WIN: &[u8] = include_bytes!("../data/youwin.ppm");
static YOU_LOSE: &[u8] = include_bytes!("../data/youlose.ppm");

/// After a mission: the original's own screen ("MISSION COMPLETED!" or
/// "YOULOSE"), why, and what next. Going on means the next mission after a
/// win, another try after a loss.
pub fn result(terminal: &mut DefaultTerminal, input: &mut Input, index: usize, won: bool, verdict: &str) -> io::Result<Pick> {
    static WIN: std::sync::OnceLock<(Vec<u8>, usize, usize)> = std::sync::OnceLock::new();
    static LOSE: std::sync::OnceLock<(Vec<u8>, usize, usize)> = std::sync::OnceLock::new();
    let img = if won { WIN.get_or_init(|| title::parse_ppm(YOU_WIN)) } else { LOSE.get_or_init(|| title::parse_ppm(YOU_LOSE)) };
    let last = index + 1 == MISSIONS.len();
    let next = match (won, last) {
        (true, false) => format!("Enter: on to mission {}, {}", MISSIONS[index + 1].number, MISSIONS[index + 1].title),
        (true, true) => {
            let act = ACTS[MISSIONS[index].act].name.split(':').next().unwrap_or("The act");
            format!("{act} is over. What comes next is not built yet. Enter or Esc: back to the title")
        }
        (false, _) => format!("Enter: try {} again    Esc: back to the title", MISSIONS[index].title),
    };
    let start = Instant::now();
    loop {
        let t = start.elapsed().as_secs_f32();
        terminal.draw(|f| {
            let area = f.area();
            f.render_widget(Block::default().style(Style::default().bg(Color::Black)), area);
            let pic = title::draw_picture(f.buffer_mut(), area, img, (t / 0.8).min(1.0), (0.8, 0.72, 0.04));
            let y = pic.y + pic.height + 1;
            let color = if won { AMBER } else { Color::Rgb(235, 70, 55) };
            centred(f, area, y, vec![Span::styled(verdict.to_string(), Style::default().fg(color).add_modifier(Modifier::BOLD))]);
            if t > 1.0 {
                centred(f, area, y + 2, vec![Span::styled(next.clone(), Style::default().fg(DIM))]);
            }
        })?;
        // a second's grace, so a key still held from the battle does not skip it
        match read_choice(input, Duration::from_millis(33))? {
            Some(go) if t > 1.0 => return Ok(if go && !(won && last) { Pick::Go } else { Pick::Back }),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Class, Outcome, Team, World};

    /// Every mission's map loads, has our HQ and supertank start, and its
    /// rules all apply (none already true before the battle).
    #[test]
    fn missions_load() {
        for m in MISSIONS {
            let (level, name, music) = map(m).unwrap();
            assert!(music.is_some(), "{name}: no music");
            assert_ne!(place(m).0, title::GOLGOTHA.0, "{name}: not on the globe");
            let w = World::from_level(&level);
            assert!(w.find(Class::Hq, Team::Player).is_some(), "{name}: no HQ");
            assert!(w.player_tank.is_some(), "{name}: no supertank start");
            for c in level.rules.win.iter().chain(&level.rules.lose) {
                assert!(!c.holds(&w), "{name}: {} holds before the battle", c.text());
            }
        }
    }

    /// A won mission opens the next and the save remembers it; nothing goes
    /// past the last mission or backwards.
    #[test]
    fn progress_is_saved() {
        let dir = std::env::temp_dir().join(format!("calvary-progress-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &dir);
        assert_eq!(reached(), 1);
        unlock_after(0);
        assert_eq!(reached(), 2.min(MISSIONS.len()));
        unlock_after(MISSIONS.len() + 5);
        assert_eq!(reached(), MISSIONS.len());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Al Basrah is won by clearing the field and lost with the HQ.
    #[test]
    fn al_basrah_rules() {
        let (level, _, _) = map(&MISSIONS[0]).unwrap();
        let mut w = World::from_level(&level);
        w.step(1.0 / 30.0);
        assert_eq!(w.outcome, Outcome::Playing);
        for e in w.entities.iter_mut().filter(|e| e.team == Team::Enemy) {
            e.alive = false;
        }
        w.step(1.0 / 30.0);
        assert_eq!(w.outcome, Outcome::Won);

        let mut w = World::from_level(&level);
        let hq = w.find(Class::Hq, Team::Player).unwrap();
        w.entities[hq].alive = false;
        w.step(1.0 / 30.0);
        assert_eq!(w.outcome, Outcome::Lost);
        assert!(w.verdict.contains("base"), "{}", w.verdict);
    }

    /// Shatt al-Arab: six gaps, only peons and bridgers, and a bridger sent
    /// up the first causeway spans its first channel.
    #[test]
    fn shatt_al_arab_needs_bridgers() {
        let (level, _, _) = map(&MISSIONS[1]).unwrap();
        let mut w = World::from_level(&level);
        assert_eq!((0..w.nodes.len()).filter(|&n| w.gap(n)).count(), 6);
        assert!(w.offers(Team::Player, "bridger") && w.offers(Team::Player, "peon_tank") && !w.offers(Team::Player, "rocket_tank"));
        assert!(w.order(Team::Player, "bridger"));
        for _ in 0..30 * 120 {
            w.step(1.0 / 30.0);
        }
        assert_eq!((0..w.nodes.len()).filter(|&n| w.gap(n)).count(), 5, "one channel bridged");
        assert_eq!(w.outcome, Outcome::Playing);
    }

    /// Balance probe for a mission: `MISSION=3 cargo test --release
    /// mission_report -- --ignored --nocapture` (default mission 2). A
    /// scripted player orders a bridger a minute where the map offers them,
    /// and every 20 s the next of the map's other units; with `SUPERTANK=1`
    /// the enemy towers and armour are cleared every minute from 2:00 and
    /// the supertank parks on the map's site to hold, and supply trucks wait
    /// at home until 2:00, standing in for the supertank's work
    /// (`CLEAR_EVERY=30`: clear every 30 s instead, a closer escort). `RUST_LOG=info` shows the battle's log,
    /// `TRUCKS=1` where the supply trucks are each minute.
    #[test]
    #[ignore]
    fn mission_report() {
        let _ = env_logger::builder().is_test(true).try_init();
        let n: usize = std::env::var("MISSION").ok().and_then(|m| m.parse().ok()).unwrap_or(2);
        let (level, _, _) = map(&MISSIONS[n - 1]).unwrap();
        let mut w = World::from_level(&level);
        let supertank = std::env::var("SUPERTANK").is_ok();
        let every: usize = std::env::var("CLEAR_EVERY").ok().and_then(|m| m.parse().ok()).unwrap_or(60);
        // when the stand-in starts (a race is decided before two minutes)
        let from: usize = std::env::var("CLEAR_FROM").ok().and_then(|m| m.parse().ok()).unwrap_or(120);
        // structures the stand-in leaves for the army to take (`SPARE=supergun`)
        let spare: Vec<String> = std::env::var("SPARE").map(|s| s.split(',').map(str::to_string).collect()).unwrap_or_default();
        let units: Vec<&'static str> = crate::world::BUILDABLE.iter().copied().filter(|k| *k != "bridger" && w.offers(Team::Player, k)).collect();
        for s in 0..900 {
            if s % 60 == 5 {
                w.order(Team::Player, "bridger");
            }
            if s % 20 == 0 && !units.is_empty() {
                w.order(Team::Player, units[(s / 20) % units.len()]);
            }
            // the supply trucks wait at home until the road is cleared
            let trucks: Vec<usize> = (0..w.entities.len()).filter(|&i| w.entities[i].kind == "car" && w.entities[i].team == Team::Player).collect();
            if supertank && s == 0 {
                w.command(&trucks, crate::world::Command::Hold);
            }
            if supertank && s == 121 {
                w.command(&trucks, crate::world::Command::Resume);
            }
            if std::env::var("TRUCKS").is_ok() && s % 60 == 0 {
                let at: Vec<String> = trucks.iter().filter(|&&i| w.entities[i].alive).map(|&i| format!("({:.0},{:.0} {:?} n{:?})", w.entities[i].x, w.entities[i].y, w.entities[i].order, w.entities[i].next_node)).collect();
                eprintln!("{s}s trucks {}", at.join(" "));
            }
            if supertank && s >= from && s % every == 0 {
                for e in w.entities.iter_mut().filter(|e| e.team.hostile(Team::Player) && matches!(e.class, Class::Tower | Class::Vehicle) && !spare.contains(&e.kind)) {
                    e.alive = false;
                }
                if let (Some(t), Some(site)) = (w.player_tank, w.sites().first().copied()) {
                    (w.entities[t].x, w.entities[t].y) = (site.1, site.2);
                }
                // and, as a player would, sends idle units back to the road,
                // then units near an enemy building at it: units leave
                // buildings alone by themselves
                let idle: Vec<usize> = (0..w.entities.len())
                    .filter(|&i| w.entities[i].alive && w.entities[i].team == Team::Player && w.entities[i].class == Class::Vehicle && w.entities[i].order == crate::world::Order::Hold && w.entities[i].kind != "car")
                    .collect();
                w.command(&idle, crate::world::Command::Resume);
                for i in 0..w.entities.len() {
                    let e = &w.entities[i];
                    if !(e.alive && e.team == Team::Player && e.class == Class::Vehicle && e.kind != "car") {
                        continue;
                    }
                    let building = (0..w.entities.len())
                        .filter(|&j| {
                            let b = &w.entities[j];
                            b.is_target() && !b.dangerous() && b.team.hostile(Team::Player) && (b.x - e.x).hypot(b.y - e.y) < 14.0
                        })
                        .min_by(|&a, &b| {
                            let d = |j: usize| (w.entities[j].x - e.x).hypot(w.entities[j].y - e.y);
                            d(a).total_cmp(&d(b))
                        });
                    if let Some(b) = building {
                        w.command(&[i], crate::world::Command::Attack(b));
                    }
                }
                // and sends the nearest unit for a relic lying loose
                for r in 0..w.entities.len() {
                    if w.entities[r].class != Class::Relic || !w.entities[r].alive || w.entities[r].carrier.is_some() {
                        continue;
                    }
                    let (rx, ry) = (w.entities[r].x, w.entities[r].y);
                    let nearest = (0..w.entities.len())
                        .filter(|&i| w.entities[i].alive && w.entities[i].team == Team::Player && w.entities[i].class == Class::Vehicle && !crate::world::flies(&w.entities[i].kind))
                        .min_by(|&a, &b| {
                            let d = |i: usize| (w.entities[i].x - rx).hypot(w.entities[i].y - ry);
                            d(a).total_cmp(&d(b))
                        });
                    if let Some(i) = nearest {
                        // by the roads: a straight line may cross a river
                        (w.entities[i].order, w.entities[i].next_node) = (crate::world::Order::Home(rx, ry), None);
                    }
                }
            }
            for _ in 0..30 {
                w.step(1.0 / 30.0);
            }
            if s % 60 == 59 {
                let units = |t: Team| w.entities.iter().filter(|e| e.alive && e.class == Class::Vehicle && (e.team == t || t == Team::Enemy && e.team.hostile(Team::Player))).count();
                let banks = |t: Team| w.entities.iter().filter(|e| e.alive && e.class == Class::Bank && (e.team == t || t == Team::Enemy && e.team.hostile(Team::Player))).count();
                let gaps = (0..w.nodes.len()).filter(|&n| w.gap(n)).count();
                let towers = w.entities.iter().filter(|e| e.alive && e.class == Class::Tower).count();
                let held: Vec<String> = w.sites().iter().map(|s| format!("{:.0}/{:.0}", s.5, s.4)).collect();
                println!(
                    "{:>3}s ours {:>2} theirs {:>2} banks {}/{} gaps {gaps} towers {towers} held {held:?} money {:?} {:?}",
                    s + 1,
                    units(Team::Player),
                    units(Team::Enemy),
                    banks(Team::Player),
                    banks(Team::Enemy),
                    w.money,
                    w.outcome
                );
            }
        }
    }
}
