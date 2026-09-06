/**
 * Reads a persisted preference from localStorage, with a default.
 *
 * @param {string} k - the storage key.
 * @param {*} d - the value to return when the key is absent.
 * @returns {*} the stored value, or `d`.
 */
const get = (k, d) => { const v = $.storage.get(k); return v == null ? d : v; };

/**
 * Builds an empty edit set (no added/removed groups or sensors, no orderings).
 *
 * @returns {object} a fresh, empty edit set.
 */
const blankEdits = () => ({ addGroups: [], addSensors: [], rmGroups: [], rmSensors: [], groupOrder: {}, sensorOrder: {} });

export const store = $.store('app', {
  state: {
    theme: get('theme', 'night'),
    locale: get('locale', 'en'),
    scenario: get('scenario', 'normal'),
    selected: null,
    editing: false,
    create: null,
    network: false,
    netInspect: null,
    netSensor: null,
    group: null,
    alarms: false,
    pairing: false,
    edits: get('edits', blankEdits()),
  },
  actions: {
    setTheme(state, v) { state.theme = v; $.storage.set('theme', v); },
    setLocale(state, v) { state.locale = v; $.storage.set('locale', v); },
    setScenario(state, v) { state.scenario = v; $.storage.set('scenario', v); state.selected = null; },
    selectSensor(state, id) { state.selected = id; },
    closeSensor(state) { state.selected = null; },
    openNetwork(state) { state.network = true; },
    closeNetwork(state) { state.network = false; state.netInspect = null; state.netSensor = null; },
    setNetInspect(state, id) { state.netInspect = id; },
    clearNetInspect(state) { state.netInspect = null; },
    setNetSensor(state, id) { state.netSensor = id; },
    clearNetSensor(state) { state.netSensor = null; },
    setGroupView(state, id) { state.group = id; },
    clearGroupView(state) { state.group = null; },
    openMeshView(state, id) { state.meshView = id; state.meshNode = null; },
    closeMeshView(state) { state.meshView = null; state.meshNode = null; },
    setMeshNode(state, id) { state.meshNode = id; },
    clearMeshNode(state) { state.meshNode = null; },
    openAlarms(state) { state.alarms = true; },
    closeAlarms(state) { state.alarms = false; },
    openPairing(state) { state.pairing = true; },
    closePairing(state) { state.pairing = false; },

    toggleEditing(state) { state.editing = !state.editing; if (!state.editing) state.create = null; },
    openCreate(state, payload) { state.create = payload; },
    closeCreate(state) { state.create = null; },

    addGroup(state, g) { state.edits.addGroups.push(g); $.storage.set('edits', state.edits); },
    removeGroup(state, id)
    {
      state.edits.addGroups = state.edits.addGroups.filter((x) => x.id !== id);
      if (!state.edits.rmGroups.includes(id)) state.edits.rmGroups.push(id);
      $.storage.set('edits', state.edits);
    },
    addSensor(state, s) { state.edits.addSensors.push(s); $.storage.set('edits', state.edits); },
    removeSensor(state, key)
    {
      state.edits.addSensors = state.edits.addSensors.filter((x) => x.groupId + '/' + x.id !== key);
      if (!state.edits.rmSensors.includes(key)) state.edits.rmSensors.push(key);
      $.storage.set('edits', state.edits);
    },
    reorderGroups(state, { orgId, ids }) { (state.edits.groupOrder ||= {})[orgId] = ids; $.storage.set('edits', state.edits); },
    reorderSensors(state, { gid, ids }) { (state.edits.sensorOrder ||= {})[gid] = ids; $.storage.set('edits', state.edits); },
    resetEdits(state) { state.edits = blankEdits(); $.storage.set('edits', state.edits); },
  },
});
