/// <reference types="cypress" />

import { get_date_range, get_dates_as_strings, get_estimates_hours, time_invested_per_day, max_hours_invested_or_estimated } from '../../src/kcf-graph'

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
      // Plus two days extra for context
      expect(dates[3]).to.eq('2022-09-04')
      expect(dates[4]).to.eq('2022-09-05')
      expect(dates.length).to.eq(5)
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
        // All days plus two extra for context
        expect(estimated_hours.min.length).to.eq(53 + 2)
      })
    })

    it('time_invested_per_day sorts tasks correctly according to time logs', function () {
      cy.fixture('tasks').then((tasks) => {
        expect(tasks.length).to.eq(11)
        const tipd = time_invested_per_day(tasks)
        expect(tipd['2022-10-19'].time_invested).to.eq(778)
        expect(tipd['2022-10-19'].tasks[0].TASK_ID).to.eq('c4cea87b7e9a0db374d6679570555e08')
      })
    })

    it('max_hours_invested_or_estimated calculates reasonable value', function () {
      cy.fixture('tasks').then((tasks) => {
        expect(tasks.length).to.eq(11)
        const date_range = get_date_range(tasks)
        const days = get_dates_as_strings(date_range.start, date_range.end)
        const tipd = time_invested_per_day(tasks)
        const max_hours = max_hours_invested_or_estimated(tasks, days, tipd)
        expect(max_hours).to.eq(53)
      })
    })
  })
})
