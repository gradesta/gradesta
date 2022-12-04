/// <reference types="cypress" />

import { get_date_range, get_dates_as_strings, get_estimates_hours } from '../../src/kcf-graph'

describe('Unit tests', function () {
  before(() => {
    // check if the import worked correctly
    expect(get_date_range, 'get_date_range').to.be.a('function')
  })

  context('kcf-graph.js', function () {
    it("get_date_range doesn't crash when an empty task list is passed.", function () {
      expect(get_date_range([]).start).to.eq((null))
      expect(get_date_range([]).end).to.eq((null))
    })

    it('get_date_range works with valid data', function () {
      cy.fixture('tasks').then((tasks) => {
        const range = get_date_range(tasks)
        expect(range.start).to.eq('2022-09-01 18:49')
        expect(range.end).to.eq('2022-10-23 00:00')
      })
    })

    it('get_dates_as_strings works with valid data (simultaneously tests get_dates_as_ms)', function () {
      const dates = get_dates_as_strings('2022-09-01 00:00', '2022-09-3 00:00')
      expect(dates[0]).to.eq('2022-09-01')
      expect(dates[1]).to.eq('2022-09-02')
      expect(dates[2]).to.eq('2022-09-03')
      expect(dates.length).to.eq(3)
    })

    it('get_estimates_hours works with valid data', function () {
      cy.fixture('tasks').then((tasks) => {
        expect(tasks.length).to.eq(11)
        const range = get_date_range(tasks)
        expect(range.start).to.eq('2022-09-01 18:49')
        expect(range.end).to.eq('2022-10-23 00:00')
        const estimated_hours = get_estimates_hours(tasks, get_dates_as_strings(range.start, range.end))
        expect(estimated_hours.min[0]).to.eq(0)
        expect(estimated_hours.min[1]).to.eq(4)
        expect(estimated_hours.max[0]).to.eq(0)
        expect(estimated_hours.max[1]).to.eq(44)
        expect(estimated_hours.min.length).to.eq(estimated_hours.max.length)
        expect(estimated_hours.min.length).to.eq(53)
      })
    })
  })
})
