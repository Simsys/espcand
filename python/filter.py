
class Filter():
    def __init__(self, pattern):
        self.zeros = 0
        self.ones = 0
        for c in pattern:
            self.zeros = self.zeros << 1
            self.ones = self.ones << 1
            if c == '1':
                self.ones |= 1
            if c == '0':
                self.zeros |= 1

    def match(self, id):
        match_pattern = (id^0b1111) & self.zeros
        if match_pattern != self.zeros:
            return False
        match_pattern = id & self.ones
        if match_pattern != self.ones:
            return False
        return True


filter = Filter("1010")
print(filter.match(0b1010))
print(filter.match(0b1011))
print(filter.match(0b1000))

filter = Filter("101x")
print(filter.match(0b1010))
print(filter.match(0b1011))
print(filter.match(0b1000))
print(filter.match(0b1110))
