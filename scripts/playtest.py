#!/usr/bin/env python3
import collections
import json
import pathlib
import re
import sys
import time


class Ron:
    def __init__(self, text):
        self.tokens = re.findall(r'"(?:\\.|[^"\\])*"|-?\d+(?:\.\d+)?(?:e[+-]?\d+)?|[A-Za-z_][A-Za-z_0-9]*|[^\s]', text)
        self.index = 0

    def pop(self):
        value = self.tokens[self.index]
        self.index += 1
        return value

    def peek(self):
        return self.tokens[self.index]

    def value(self):
        token = self.pop()
        if token.startswith('"'):
            token = token.replace("\\'", "'").replace('\\0', '\\u0000')
            token = re.sub(r'\\u\{([0-9a-fA-F]+)\}', lambda m: json.dumps(chr(int(m[1], 16)))[1:-1], token)
            return json.loads(token)
        if re.fullmatch(r'-?\d+(?:\.\d+)?(?:e[+-]?\d+)?', token):
            return float(token) if '.' in token or 'e' in token else int(token)
        if token in ('true', 'false', 'None'):
            return {'true': True, 'false': False, 'None': None}[token]
        if token in ('(', '[', '{'):
            end = {'(': ')', '[': ']', '{': '}'}[token]
            if self.peek() == end:
                self.pop()
                return []
            record = self.index + 1 < len(self.tokens) and self.tokens[self.index + 1] == ':'
            result = {} if record else []
            while self.peek() != end:
                if record:
                    key = self.pop()
                    if key.startswith('"'):
                        key = json.loads(key)
                    assert self.pop() == ':'
                    result[key] = self.value()
                else:
                    result.append(self.value())
                if self.peek() == ',':
                    self.pop()
                else:
                    break
            assert self.pop() == end
            return result
        if self.index < len(self.tokens) and self.peek() == '(':
            values = self.value()
            if token == 'Some':
                return values[0]
            return {'type': token, 'value': values}
        return token


def load_ron(path):
    return Ron(pathlib.Path(path).read_text()).value()


class Game:
    def __init__(self, directory):
        self.directory = pathlib.Path(directory)
        self.state = load_ron(self.directory / 'state.ron')
        self.modified = (self.directory / 'state.ron').stat().st_mtime_ns

    def refresh(self):
        path = self.directory / 'state.ron'
        modified = path.stat().st_mtime_ns
        if modified != self.modified:
            self.state = load_ron(path)
            self.modified = modified
        return self.state

    def send(self, frames=1, keys='', interval=0, capture=False, timeout=650):
        self.refresh()
        command_id = self.state['id'] + 1
        command = f'{command_id} {frames} {interval} {keys} {"capture" if capture else ""}\n'
        state_path = self.directory / 'state.ron'
        modified = state_path.stat().st_mtime_ns
        temporary = self.directory / 'command.tmp'
        temporary.write_text(command)
        temporary.replace(self.directory / 'command.txt')
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            time.sleep(0.02)
            if state_path.stat().st_mtime_ns == modified:
                continue
            self.refresh()
            if self.state['id'] == command_id:
                with (self.directory / 'timeline.jsonl').open('a') as timeline:
                    timeline.write(json.dumps({'at': time.time(), 'id': command_id,
                                               'frame': self.state['frame'],
                                               'map': self.state['map'],
                                               'timer': self.state['timer']}) + '\n')
                return self.state
        raise TimeoutError(command)

    def tap(self, key='enter', capture=False):
        self.send(1)
        return self.send(14, key, interval=14, capture=capture)

    def settle(self, frames=120, capture=False):
        return self.send(frames, capture=capture)

    def advance(self, limit=240):
        for _ in range(limit):
            s = self.state
            if s['choice'] or s['number'] is not None or s['battle'] or s['shop'] or s['menu']:
                return s
            if s['transition'] or s['dialogue'] and not s['dialogue_ready']:
                self.settle(60)
            elif s['dialogue']:
                self.tap()
                self.settle(12)
            elif s['running_event'] is not None:
                self.settle(60)
            else:
                return s
        raise RuntimeError('dialogue advance limit')

    def choose(self, index):
        assert self.state['choice'], 'no choice active'
        assert 0 <= index < len(self.state['options'])
        original_options = self.state['options'][:]
        self.settle(180)
        for _ in range(8):
            while self.state['choice_cursor'] != index:
                before = self.state['choice_cursor']
                self.tap('down')
                if self.state['choice_cursor'] == before:
                    self.settle(60)
            self.tap('enter', capture=True)
            if self.state['choice'] and self.state['options'] != original_options:
                return self.state
            if not self.state['choice']:
                self.settle(12)
                return self.advance()
            self.settle(60)
        raise RuntimeError('choice did not accept input')

    def door(self, event_id):
        original_map = self.state['map']
        event = next(e for e in self.state['events'] if e['id'] == event_id)
        transfer = any(c['code'] == 10810 for c in event['commands'])
        for _ in range(4):
            self.approach(event_id)
            if self.state['map'] == original_map and not self.state['dialogue']:
                self.walk(event['x'], event['y'])
            self.settle(90)
            self.advance()
            if (not transfer or self.state['map'] != original_map
                    or self.state['choice'] or self.state['battle']):
                return self.state
        raise RuntimeError(f'transfer event {event_id} did not leave map {original_map}')

    def enter_number(self, value):
        assert self.state['number'] is not None, 'no numeric prompt active'
        digits = self.state['number'][2]
        assert 0 <= value < 10 ** digits
        target = str(value).zfill(digits)
        self.settle(60)
        for _ in range(60):
            current, cursor, count = self.state['number']
            assert count == digits
            actual = str(current).zfill(digits)
            if actual == target:
                self.tap()
                self.settle(30)
                return self.advance()
            if actual[cursor] == target[cursor]:
                self.tap('left')
            else:
                delta = (int(target[cursor]) - int(actual[cursor])) % 10
                self.tap('up' if delta <= 5 else 'down')
        raise RuntimeError('numeric cursor did not reach selection')

    def summary(self):
        fields = ['id', 'frame', 'map', 'player', 'title', 'transition', 'running_event',
                  'dialogue', 'dialogue_ready', 'text', 'choice', 'options', 'choice_cursor',
                  'number', 'menu', 'menu_state', 'shop', 'save_allowed', 'battle',
                  'battle_phase', 'battle_menu', 'battle_cursor', 'battle_turn', 'battle_round',
                  'fighters', 'enemies', 'party', 'levels', 'vitals', 'gold', 'items', 'timer']
        result = {key: self.state[key] for key in fields}
        for key in ['shop_state', 'conditions', 'battle_states']:
            if key in self.state:
                result[key] = self.state[key]
        if self.state['battle']:
            result['battle_log'] = self.state['battle_log'][-10:]
        return result

    def path(self, destinations, avoid=()):
        s = self.state
        start = tuple(s['player'][:2])
        destinations = set(destinations)
        avoid = set(avoid)
        avoid.update((e['x'], e['y']) for e in s['events']
                     if (e['x'], e['y']) not in destinations
                     and e['trigger'] in (1, 2)
                     and any(c['code'] == 10810 for c in e['commands']))
        seen = {start: None}
        queue = collections.deque([start])
        directions = [(1, 0, -1, 'up'), (2, 1, 0, 'right'), (4, 0, 1, 'down'), (8, -1, 0, 'left')]
        while queue:
            position = queue.popleft()
            if position in destinations:
                path = []
                while seen[position] is not None:
                    previous, key = seen[position]
                    path.append(key)
                    position = previous
                return path[::-1]
            x, y = position
            if not (0 <= x < s['width'] and 0 <= y < s['height']):
                continue
            mask = s['edges'][y * s['width'] + x]
            for bit, dx, dy, key in directions:
                target = (x + dx, y + dy)
                if mask & bit and target not in seen and target not in avoid:
                    seen[target] = (position, key)
                    queue.append(target)
        raise ValueError(f'no path from {start} to {destinations}')

    def walk(self, x, y, avoid=()):
        original_map = self.state['map']
        visited = collections.Counter()
        blocked = 0
        movement_frames = getattr(self, '_movement_frames', {})
        self._movement_frames = movement_frames
        for _ in range(300):
            if (self.state['map'] != original_map or self.state['dialogue']
                    or self.state['choice'] or self.state['battle']
                    or self.state['running_event'] is not None):
                return self.state
            while self.state['player'][3]:
                self.settle(2)
                if self.state['map'] != original_map or self.state['dialogue']:
                    return self.state
            if tuple(self.state['player'][:2]) == (x, y):
                return self.state
            signature = (tuple(self.state['player'][:2]), tuple(map(tuple, self.state['switches'])))
            visited[signature] += 1
            if visited[signature] > 8:
                raise RuntimeError('route cycles across changing event pages; use an explicit detour')
            try:
                path = self.path([(x, y)], avoid)
                blocked = 0
            except ValueError:
                blocked += 1
                if self.open_route_door(x, y, avoid):
                    blocked = 0
                    continue
                if blocked >= 8:
                    raise
                self.settle(60)
                continue
            before = self.state['player'][:2]
            count = 1
            segment_limit = getattr(self, 'movement_segment_limit', 16)
            while (original_map in movement_frames
                   and count < min(segment_limit, len(path)) and path[count] == path[0]):
                count += 1
            dx, dy = {'up': (0, -1), 'right': (1, 0), 'down': (0, 1), 'left': (-1, 0)}[path[0]]
            touch = {(e['x'], e['y']) for e in self.state['events']
                     if e['trigger'] in (1, 2) and any(c['code'] for c in e['commands'])}
            for step in range(1, count + 1):
                if (before[0] + dx * step, before[1] + dy * step) in touch:
                    count = step
                    break
            started = self.state['frame']
            rate = movement_frames.get(original_map, 8)
            self.send((count - 1) * rate + 1, path[0])
            self.settle(rate)
            if self.state['running_event'] is not None:
                return self.state
            if original_map not in movement_frames:
                while (self.state['player'][3] and self.state['map'] == original_map
                       and not self.state['dialogue']):
                    self.settle(4)
                if (self.state['map'] == original_map
                        and self.state['player'][:2] == [before[0] + dx, before[1] + dy]
                        and not self.state['running_event']):
                    elapsed = self.state['frame'] - started
                    movement_frames[original_map] = min((4, 8, 16, 32, 64),
                                                       key=lambda frames: abs(frames - elapsed))
            if self.state['player'][:2] == before and not self.state['running_event']:
                self.settle(30)
                self.send(1, path[0])
                self.settle(8)
                if self.state['player'][:2] == before and not self.state['running_event']:
                    raise RuntimeError(f'movement blocked at {before} via {path[0]}')
        raise RuntimeError('walk limit')

    def open_route_door(self, x, y, avoid=()):
        candidates = []
        for event in self.state['events']:
            commands = event['commands']
            action = event['layer'] == 1 and event['trigger'] == 0
            proximity = (event['layer'] != 1 and event['trigger'] == 1
                         and all(c['code'] in (0, 10210, 11550) for c in commands))
            if (not (action or proximity)
                    or not any(c['code'] == 11550 and c['string'].startswith('Door') for c in commands)
                    or any(c['code'] in (10110, 10140, 10710, 10810) for c in commands)):
                continue
            ex, ey = event['x'], event['y']
            targets = [(ex, ey)] if proximity else [(ex, ey + 1), (ex - 1, ey), (ex, ey - 1), (ex + 1, ey)]
            for target in targets:
                try:
                    path = self.path([target], avoid)
                    candidates.append((abs(ex - x) + abs(ey - y), len(path), event['id'], target))
                except ValueError:
                    pass
        if not candidates:
            return False
        _, _, event_id, _ = min(candidates)
        before = (self.state['player'][:2], self.state['switches'])
        event = next(e for e in self.state['events'] if e['id'] == event_id)
        self.approach(event_id, interact=event['trigger'] == 0, avoid=avoid)
        self.settle(90)
        self.advance()
        return before != (self.state['player'][:2], self.state['switches'])

    def approach(self, event_id, interact=True, avoid=()):
        original_map = self.state['map']
        event = next(e for e in self.state['events'] if e['id'] == event_id)
        x, y = event['x'], event['y']
        if event['layer'] != 1:
            target = (x, y)
        else:
            targets = [(x, y + 1), (x - 1, y), (x, y - 1), (x + 1, y)]
            for attempt in range(9):
                paths = []
                for target in targets:
                    try:
                        paths.append((len(self.path([target], avoid)), target))
                    except ValueError:
                        pass
                if paths:
                    break
                if self.open_route_door(x, y, avoid):
                    return self.approach(event_id, interact=interact, avoid=avoid)
                if attempt == 8:
                    raise ValueError(f'cannot approach event {event_id}')
                self.settle(60)
                if (self.state['map'] != original_map or self.state['dialogue']
                        or self.state['choice'] or self.state['battle']):
                    return self.state
            target = min(paths)[1]
        self.walk(*target, avoid=avoid)
        if (self.state['map'] != original_map or self.state['dialogue']
                or self.state['choice'] or self.state['battle']
                or self.state['running_event'] is not None
                or tuple(self.state['player'][:2]) != target):
            return self.state
        if interact:
            px, py = self.state['player'][:2]
            dx, dy = x - px, y - py
            if dx or dy:
                key = 'right' if dx > 0 else 'left' if dx < 0 else 'down' if dy > 0 else 'up'
                self.send(1, key)
                self.settle(8)
            self.tap()
            self.settle(60)
        return self.state

    def events(self):
        return [{'id': e['id'], 'name': e['name'], 'xy': [e['x'], e['y']],
                 'trigger': e['trigger'], 'layer': e['layer'], 'graphic': e['graphic'],
                 'text': [c['string'] for c in e['commands'] if c['code'] in [10110, 20110, 20140]],
                 'transfer': [c['params'] for c in e['commands'] if c['code'] == 10810]}
                for e in self.state['events'] if e['commands'] or e['graphic']]

    def trade(self, item_id, index, count=1, buy=True):
        if not self.state['shop'] or 'shop_state' not in self.state:
            raise RuntimeError('trade requires an open shop with cursor observation')
        mode, command = ('Buy', 0) if buy else ('Sell', 1)
        before = dict(self.state['items']).get(item_id, 0)

        def step(key='enter'):
            self.tap(key)
            self.settle(30)

        for _ in range(30):
            phase = self.state['shop_state']
            if phase.startswith(mode + ' {'):
                break
            if phase.startswith('Command'):
                cursor = int(re.search(r'cursor: (\d+)', phase)[1])
                step('enter' if cursor == command else 'up' if cursor > command else 'down')
            elif phase.startswith(('Bought', 'Sold')):
                self.settle(60)
            else:
                raise RuntimeError(f'unexpected shop phase: {phase}')
        else:
            raise RuntimeError('shop list did not open')
        for _ in range(40):
            cursor = int(re.search(r'cursor: (\d+)', self.state['shop_state'])[1])
            if cursor == index:
                break
            key = 'down' if index > cursor else 'up'
            if not buy and index % 2 != cursor % 2:
                key = 'right' if index % 2 else 'left'
            step(key)
        else:
            raise RuntimeError('shop cursor did not reach item')
        step()
        phase = self.state['shop_state']
        if not phase.startswith('Number') or f'item_id: {item_id},' not in phase:
            raise RuntimeError(f'shop selected an unexpected item: {phase}')
        maximum = int(re.search(r'max: (\d+)', phase)[1])
        if not 1 <= count <= maximum:
            raise ValueError(f'quantity {count} exceeds shop maximum {maximum}')
        for _ in range(100):
            quantity = int(re.search(r'count: (\d+)', self.state['shop_state'])[1])
            if quantity == count:
                break
            step('right' if count > quantity else 'left')
        else:
            raise RuntimeError('shop quantity did not reach selection')
        step()
        self.settle(90)
        expected = before + count if buy else before - count
        assert dict(self.state['items']).get(item_id, 0) == expected

    def battle_cursor(self, index, count=None):
        for _ in range(30):
            cursor = self.state['battle_cursor']
            if cursor == index:
                return
            if self.state['battle_menu'] in (1, 2):
                if count is not None and count % 2 and cursor == count - 1 and index % 2:
                    key = 'up'
                else:
                    key = ('right' if index % 2 else 'left') if index % 2 != cursor % 2 else 'down' if index > cursor else 'up'
            else:
                key = 'down'
            self.tap(key)
        raise RuntimeError('battle cursor did not reach selection')

    def fight(self, skills=False, soften_to=None, stop_after_encounter=False, restore_sp=True, defend=False):
        data_skills = load_ron('assets/skills.ron')
        data_items = load_ron('assets/items.ron')
        selected_skill = None
        selected_item = None
        selected_target = 0
        planned_hp = {}
        reserved_items = collections.Counter()
        for _ in range(600):
            s = self.state
            if s['choice'] or s['number'] is not None:
                return s
            if not s['battle']:
                if stop_after_encounter:
                    self.settle(1, capture=True)
                    return self.state
                self.advance()
                if self.state['battle']:
                    continue
                self.settle(30, capture=True)
                return self.state
            if s['dialogue']:
                self.tap() if s['dialogue_ready'] else self.settle(60)
            elif s['battle_phase'] == 'PartyCommand':
                planned_hp.clear()
                reserved_items.clear()
                self.battle_cursor(0)
                self.tap()
            elif s['battle_phase'] == 'Command':
                level = s['battle_menu']
                actor = s['fighters'][s['battle_turn']]
                if defend:
                    assert level == 0, 'defence requires the command menu'
                    self.battle_cursor(2)
                    self.tap()
                    continue
                if level == 0:
                    selected_skill = selected_item = None
                    selected_target = s['battle_turn']
                    known = dict(s['skills'])[actor[0]]
                    alive = [(planned_hp.get(i, f[2]) / f[3], i)
                             for i, f in enumerate(s['fighters']) if planned_hp.get(i, f[2]) > 0]
                    injured, weakest = min(alive)
                    fallen = [i for i, f in enumerate(s['fighters']) if planned_hp.get(i, f[2]) == 0]
                    inventory = {item: count - reserved_items[item] for item, count in s['items']}
                    potions = [item for item in data_items
                               if inventory.get(item['id'], 0) > 0 and item['item_type'] == 6
                               and not item['only_field'] and not item['ko_only']
                               and (item['recover_hp'] > 0 or item['recover_hp_rate'] > 0)]
                    sp_potions = [item for item in data_items
                                  if inventory.get(item['id'], 0) > 0 and item['item_type'] == 6
                                  and not item['only_field'] and not item['ko_only']
                                  and (item['recover_sp'] > 0 or item['recover_sp_rate'] > 0)]
                    phoenix_target = any(enemy[1] >= 800 for enemy in s['enemies'])
                    phoenix_refill = (actor[0] == 1 and 6 in known and phoenix_target and any(
                        actor[4] + item['recover_sp']
                        + actor[5] * item['recover_sp_rate'] // 100 >= 300
                        for item in sp_potions))
                    sp_threshold = (300 if phoenix_refill else 60 if actor[0] == 1 and 4 in known
                                    else 20 if actor[0] == 1 else 40 if 33 in known else 15)
                    if fallen and 35 in known and actor[4] >= 80:
                        selected_skill = 35
                    elif fallen and 11 in known and actor[4] >= 50:
                        selected_skill, selected_target = 11, fallen[0]
                    elif fallen and inventory.get(112, 0) > 0:
                        selected_item, selected_target = 112, fallen[0]
                    elif sum(ratio < 0.7 for ratio, _ in alive) >= 2 and 9 in known and actor[4] >= 60:
                        selected_skill = 9
                    elif sum(ratio < 0.7 for ratio, _ in alive) >= 2 and 33 in known and actor[4] >= 40:
                        selected_skill = 33
                    elif injured < 0.5 and 37 in known and actor[4] >= 40:
                        selected_skill, selected_target = 37, weakest
                    elif (injured < 0.65 and 8 in known and actor[4] >= 40
                          and s['fighters'][weakest][3] - planned_hp.get(weakest, s['fighters'][weakest][2]) > 50):
                        selected_skill, selected_target = 8, weakest
                    elif injured < 0.65 and 7 in known and actor[4] >= 15:
                        selected_skill, selected_target = 7, weakest
                    elif injured < 0.5 and potions:
                        missing = s['fighters'][weakest][3] - planned_hp.get(weakest, s['fighters'][weakest][2])
                        maximum = s['fighters'][weakest][3]
                        selected_item = min(potions, key=lambda item: abs(
                            item['recover_hp'] + maximum * item['recover_hp_rate'] // 100 - missing))['id']
                        selected_target = weakest
                    elif (skills and restore_sp
                          and actor[4] < sp_threshold
                          and actor[5] >= 75
                          and (actor[0] == 1 or any(i in known for i in (7, 8, 9, 33, 35, 37)))
                          and sp_potions):
                        missing_sp = actor[5] - actor[4]
                        selected_item = min(sp_potions, key=lambda item: abs(
                            item['recover_sp'] + actor[5] * item['recover_sp_rate'] // 100
                            - missing_sp))['id']
                    elif skills and actor[0] == 2 and 14 in known and actor[4] >= 333:
                        selected_skill = 14
                    elif skills and actor[0] == 1 and 6 in known and actor[4] >= 300 and phoenix_target:
                        selected_skill = 6
                    elif skills and actor[0] == 8 and 42 in known and actor[4] >= 75:
                        selected_skill = 42
                    elif skills and actor[0] == 8 and 41 in known and actor[4] >= 50:
                        selected_skill = 41
                    elif (skills and actor[0] == 6 and 36 in known and actor[4] >= 100
                          and (sum(e[1] > 0 for e in s['enemies']) > 1 or max(e[1] for e in s['enemies']) > 400)):
                        selected_skill = 36
                    elif (skills and actor[0] == 1 and actor[4] >= 20
                          and (soften_to is None or max(e[1] for e in s['enemies']) <= soften_to)):
                        if 2 in known and actor[4] >= 35 and sum(e[1] > 0 for e in s['enemies']) > 1:
                            selected_skill = 2
                        elif 4 in known and actor[4] >= 60 and max(e[1] for e in s['enemies']) > 150:
                            selected_skill = 4
                        elif 1 in known:
                            selected_skill = 1
                    if selected_skill in (9, 33, 35):
                        for index, recipient in enumerate(s['fighters']):
                            hp = planned_hp.get(index, recipient[2])
                            if selected_skill in (9, 33) and hp > 0 or selected_skill == 35 and hp == 0:
                                planned_hp[index] = min(recipient[3], hp + (200 if selected_skill == 9 else 100))
                    elif selected_item or selected_skill in (7, 8, 11, 37):
                        recipient = s['fighters'][selected_target]
                        hp = planned_hp.get(selected_target, recipient[2])
                        if selected_item:
                            medicine = next(item for item in data_items if item['id'] == selected_item)
                            healed = medicine['recover_hp'] + recipient[3] * medicine['recover_hp_rate'] // 100
                            reserved_items[selected_item] += 1
                        else:
                            healed = next(skill['power'] for skill in data_skills if skill['id'] == selected_skill)
                        planned_hp[selected_target] = min(recipient[3], hp + healed)
                    self.battle_cursor(3 if selected_item else 1 if selected_skill else 0)
                    self.tap()
                elif level == 1:
                    known = dict(s['skills'])[actor[0]]
                    choices = [x['id'] for x in data_skills if x['id'] in known]
                    self.battle_cursor(choices.index(selected_skill), len(choices))
                    self.tap()
                elif level == 2:
                    inventory = dict(s['items'])
                    choices = [x['id'] for x in data_items if inventory.get(x['id'], 0)]
                    self.battle_cursor(choices.index(selected_item), len(choices))
                    self.tap()
                elif level == 3:
                    living = [e for e in s['enemies'] if e[1] > 0]
                    self.battle_cursor(min((e[1], i) for i, e in enumerate(living))[1])
                    self.tap()
                else:
                    self.battle_cursor(selected_target)
                    self.tap()
            else:
                self.send(120, 'shift')
        raise RuntimeError('battle did not finish')


def main():
    game = Game(sys.argv[1])
    action = sys.argv[2] if len(sys.argv) > 2 else 'state'
    if action == 'send':
        game.send(int(sys.argv[3]), ' '.join(sys.argv[5:]), int(sys.argv[4]), capture=True)
    elif action == 'tap':
        game.tap(sys.argv[3] if len(sys.argv) > 3 else 'enter')
        game.settle(90, capture=True)
    elif action == 'walk':
        game.walk(int(sys.argv[3]), int(sys.argv[4]))
        game.settle(1, capture=True)
    elif action == 'event':
        game.approach(int(sys.argv[3]))
        game.settle(1, capture=True)
    elif action == 'events':
        print(json.dumps(game.events(), ensure_ascii=False))
        return
    elif action == 'advance':
        game.advance()
        game.settle(1, capture=True)
    elif action == 'choose':
        game.choose(int(sys.argv[3]))
        game.settle(1, capture=True)
    elif action == 'door':
        game.door(int(sys.argv[3]))
        game.settle(1, capture=True)
    elif action == 'fight':
        game.fight(skills='skills' in sys.argv[3:])
    print(json.dumps(game.summary(), ensure_ascii=False))


if __name__ == '__main__':
    main()
